//! JSON canonicalization following RFC 8785 (JCS) for the value subset we use.
//!
//! Supported: null, booleans, integers within the I-JSON safe range
//! (|n| <= 2^53 - 1), strings, arrays, objects. Floating-point numbers are
//! rejected on purpose: our formats never need them, and their canonical form
//! is the main source of cross-implementation divergence.

use serde_json::Value;
use std::fmt;

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991; // 2^53 - 1

#[derive(Debug, PartialEq, Eq)]
pub enum CanonError {
    /// A non-integer number was found.
    NonIntegerNumber,
    /// An integer outside the exactly-representable range was found.
    IntegerOutOfRange,
}

impl fmt::Display for CanonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonIntegerNumber => write!(f, "non-integer numbers are not allowed"),
            Self::IntegerOutOfRange => write!(f, "integer outside the safe range (+/- 2^53-1)"),
        }
    }
}

impl std::error::Error for CanonError {}

/// Serialize `value` to its canonical JSON text.
pub fn canonicalize(value: &Value) -> Result<String, CanonError> {
    let mut out = String::new();
    write_value(value, &mut out)?;
    Ok(out)
}

fn write_value(value: &Value, out: &mut String) -> Result<(), CanonError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&i) {
                    return Err(CanonError::IntegerOutOfRange);
                }
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                if u > MAX_SAFE_INTEGER as u64 {
                    return Err(CanonError::IntegerOutOfRange);
                }
                out.push_str(&u.to_string());
            } else {
                return Err(CanonError::NonIntegerNumber);
            }
        }
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            // RFC 8785 §3.2.3: sort keys by their UTF-16 code units.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                if let Some(v) = map.get(key.as_str()) {
                    write_value(v, out)?;
                }
            }
            out.push('}');
        }
    }
    Ok(())
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    #[test]
    fn sorts_keys_and_is_compact() {
        let v = json!({"b": 1, "a": [true, null, "x"], "c": {"z": 0, "y": 1}});
        assert_eq!(
            canonicalize(&v).unwrap(),
            r#"{"a":[true,null,"x"],"b":1,"c":{"y":1,"z":0}}"#
        );
    }

    #[test]
    fn escapes_control_and_quote_characters_only() {
        let v = json!("a\"b\\c\n\t\u{1}\u{7f}é/");
        assert_eq!(
            canonicalize(&v).unwrap(),
            "\"a\\\"b\\\\c\\n\\t\\u0001\u{7f}é/\""
        );
    }

    #[test]
    fn orders_keys_by_utf16_code_units_not_scalar_values() {
        // U+10000 is encoded as the surrogate pair D800 DC00, which sorts
        // BEFORE U+FFFF in UTF-16 order, but AFTER it in scalar order.
        let v = json!({"\u{ffff}": 1, "\u{10000}": 2});
        assert_eq!(
            canonicalize(&v).unwrap(),
            "{\"\u{10000}\":2,\"\u{ffff}\":1}"
        );
    }

    #[test]
    fn rejects_floats_and_unsafe_integers() {
        assert_eq!(canonicalize(&json!(1.5)), Err(CanonError::NonIntegerNumber));
        assert_eq!(
            canonicalize(&json!(9_007_199_254_740_992_u64)),
            Err(CanonError::IntegerOutOfRange)
        );
        assert_eq!(
            canonicalize(&json!(-9_007_199_254_740_992_i64)),
            Err(CanonError::IntegerOutOfRange)
        );
        assert!(canonicalize(&json!(9_007_199_254_740_991_u64)).is_ok());
    }

    fn arb_json() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            (-1_000_000i64..1_000_000).prop_map(|n| json!(n)),
            ".{0,12}".prop_map(Value::String),
        ];
        leaf.prop_recursive(4, 32, 6, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..5).prop_map(Value::Array),
                prop::collection::hash_map(".{0,6}", inner, 0..5)
                    .prop_map(|m| Value::Object(m.into_iter().collect())),
            ]
        })
    }

    proptest! {
        // Canonical text parses back to a value that canonicalizes identically.
        #[test]
        fn canonicalization_is_idempotent(v in arb_json()) {
            let once = canonicalize(&v).unwrap();
            let reparsed: Value = serde_json::from_str(&once).unwrap();
            prop_assert_eq!(canonicalize(&reparsed).unwrap(), once);
        }

        // Insertion order of object keys never changes the output.
        #[test]
        fn key_insertion_order_is_irrelevant(entries in prop::collection::vec((".{0,5}", -50i64..50), 0..8)) {
            let mut forward = serde_json::Map::new();
            for (k, n) in &entries { forward.insert(k.clone(), json!(n)); }
            let mut backward = serde_json::Map::new();
            for (k, _) in entries.iter().rev() {
                if let Some(n) = forward.get(k) { backward.insert(k.clone(), n.clone()); }
            }
            prop_assert_eq!(
                canonicalize(&Value::Object(forward)).unwrap(),
                canonicalize(&Value::Object(backward)).unwrap()
            );
        }
    }
}
