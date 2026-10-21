//! Differential test of the canonical JSON writer against an independent
//! implementation: the ECMAScript JSON serializer that RFC 8785 (JCS) is
//! defined in terms of, run by `node`.
//!
//! The reference serializer sorts keys explicitly by UTF-16 code units (the
//! default JS string order) and serializes strings and integers with
//! `JSON.stringify`. It does not rely on object enumeration order (which
//! reorders integer-like keys in JS).
//!
//! Set `OPSHERO_REQUIRE_NODE=1` (CI does) to make a missing `node` a failure
//! instead of a loud skip.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use opshero_core::canon::canonicalize;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

const REFERENCE: &str = r#"
const ser = (v) => {
  if (v === null) return 'null';
  if (Array.isArray(v)) return '[' + v.map(ser).join(',') + ']';
  switch (typeof v) {
    case 'boolean': case 'number': case 'string': return JSON.stringify(v);
    case 'object': return '{' + Object.keys(v).sort().map(k => JSON.stringify(k) + ':' + ser(v[k])).join(',') + '}';
  }
  throw new Error('unsupported');
};
let input = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', d => input += d);
process.stdin.on('end', () => {
  const docs = JSON.parse(input);
  process.stdout.write(JSON.stringify(docs.map(ser)));
});
"#;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn reference(docs: &[Value]) -> Vec<String> {
    let mut child = Command::new("node")
        .args(["-e", REFERENCE])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let payload = serde_json::to_vec(docs).unwrap();
    child.stdin.take().unwrap().write_all(&payload).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "reference implementation failed");
    serde_json::from_slice(&out.stdout).unwrap()
}

fn nasty_char() -> impl Strategy<Value = char> {
    prop_oneof![
        Just('"'),
        Just('\\'),
        Just('\n'),
        Just('\r'),
        Just('\t'),
        Just('\u{0}'),
        Just('\u{8}'),
        Just('\u{c}'),
        Just('\u{1f}'),
        Just('\u{7f}'),
        Just('\u{2028}'),
        Just('\u{2029}'),
        Just('\u{d7ff}'),
        Just('\u{e000}'),
        Just('\u{ffff}'),
        Just('\u{1f600}'),
        Just('\u{10000}'),
        Just('\u{10ffff}'),
        Just('é'),
        Just('/'),
        Just('<'),
        Just('1'),
        Just('a'),
        any::<char>(),
    ]
}

fn text() -> impl Strategy<Value = String> {
    proptest::collection::vec(nasty_char(), 0..6).prop_map(|c| c.into_iter().collect())
}

fn int() -> impl Strategy<Value = i64> {
    const MAX: i64 = 9_007_199_254_740_991;
    prop_oneof![
        Just(0),
        Just(-1),
        Just(MAX),
        Just(-MAX),
        Just(MAX - 1),
        -1000i64..1000,
        -MAX..=MAX,
    ]
}

fn json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        int().prop_map(|n| Value::Number(n.into())),
        text().prop_map(Value::String),
    ];
    leaf.prop_recursive(4, 48, 5, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..5).prop_map(Value::Array),
            proptest::collection::vec((text(), inner), 0..5)
                .prop_map(|kv| Value::Object(kv.into_iter().collect())),
        ]
    })
}

#[test]
fn canonical_json_matches_the_ecmascript_reference() {
    if !node_available() {
        assert!(
            std::env::var("OPSHERO_REQUIRE_NODE").is_err(),
            "node is required (OPSHERO_REQUIRE_NODE is set)"
        );
        eprintln!("SKIPPED: `node` not found; the differential test did not run");
        return;
    }
    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
        cases: 1,
        ..ProptestConfig::default()
    });
    let batch = proptest::collection::vec(json(), 4000..=4000);
    let docs = batch.new_tree(&mut runner).unwrap().current();
    let expected = reference(&docs);
    assert_eq!(expected.len(), docs.len());
    let mut mismatches = Vec::new();
    let mut reference_misparses = 0usize;
    for (doc, want) in docs.iter().zip(&expected) {
        let got = canonicalize(doc).unwrap();
        if &got == want {
            continue;
        }
        // A disagreement only counts if a cold, isolated reference run
        // reproduces it. Observed with Node 24.19 (V8 13.6): inside a long
        // batch, `JSON.parse` sometimes returns the key `\\` for an object key
        // that is a single escaped character (`"\r"`, `"\""`...). The same
        // document parsed alone, and an independent Python implementation,
        // agree with our output. See RFC-0005 section 6.
        let cold = reference(std::slice::from_ref(doc)).remove(0);
        if cold == got {
            reference_misparses += 1;
        } else {
            mismatches.push((doc.to_string(), got, cold));
        }
    }
    if reference_misparses > 0 {
        eprintln!(
            "note: the batch reference run misparsed {reference_misparses} of {} documents (cold re-runs agree with canonicalize)",
            docs.len()
        );
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} documents differ; first: {:?}",
        mismatches.len(),
        docs.len(),
        mismatches.first()
    );
}

/// Hand-picked documents that are classic JCS traps: key order across the
/// BMP/astral boundary (UTF-16 order differs from code point order), integer-like
/// keys, escapes, and the limits of the supported integer range.
#[test]
fn known_traps_match_the_reference() {
    if !node_available() {
        assert!(std::env::var("OPSHERO_REQUIRE_NODE").is_err());
        eprintln!("SKIPPED: `node` not found");
        return;
    }
    let docs: Vec<Value> = [
        r#"{"\ud83d\ude00":1,"\ue000":2,"\uffff":3,"a":4}"#, // U+1F600 sorts before U+E000 in UTF-16
        r#"{"10":1,"9":2,"1":3,"":4,"a":5}"#,
        r#"{"\u0000":1,"\u001f":2,"\u007f":3,"\u2028":4}"#,
        r#"["\"","\\","\/","\b\f\n\r\t","\u00e9","\ud83d\ude00"]"#,
        r#"[9007199254740991,-9007199254740991,0,1,-1]"#,
        r#"{"a":{"b":[{"c":null}],"B":true},"A":false}"#,
        r#"{}"#,
        r#"[]"#,
        r#""""#,
    ]
    .iter()
    .map(|t| serde_json::from_str(t).unwrap())
    .collect();
    let expected = reference(&docs);
    for (doc, want) in docs.iter().zip(&expected) {
        assert_eq!(&canonicalize(doc).unwrap(), want, "document {doc}");
    }
}

/// Values outside the supported subset must be refused, never rounded.
#[test]
fn unsupported_numbers_are_refused_not_rounded() {
    for t in [
        "1.5",
        "1e3",
        "9007199254740992",
        "-9007199254740992",
        "18446744073709551615",
        "0.0",
    ] {
        let v: Value = serde_json::from_str(t).unwrap();
        assert!(canonicalize(&v).is_err(), "{t} must be refused");
    }
}
