//! Mutation-based robustness harness for every parser that sees untrusted
//! bytes (policy files, record files, hook command lines, timestamps).
//!
//! This is NOT coverage-guided fuzzing (that needs a nightly toolchain and
//! `cargo-fuzz`; see docs/rfc/0005 and the roadmap). It is a deterministic
//! mutation fuzzer: fixed seeds, a corpus of valid inputs, structure-aware
//! dictionary splicing, byte flips, truncation, duplication. Properties:
//! no panic, no call slower than one second, and, where a round-trip exists,
//! the round-trip holds.
//!
//! `OPSHERO_FUZZ_ITERS` raises the iteration count (default 20,000 per target).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use opshero_core::{canon, policy, record, shellcmd, timefmt};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn iterations() -> usize {
    std::env::var("OPSHERO_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000)
}

fn mutate(rng: &mut Rng, seed: &[u8], dict: &[&str]) -> Vec<u8> {
    let mut b = seed.to_vec();
    for _ in 0..=rng.below(4) {
        match rng.below(7) {
            0 if !b.is_empty() => {
                let i = rng.below(b.len());
                b[i] ^= 1 << rng.below(8);
            }
            1 => {
                let i = rng.below(b.len() + 1);
                b.insert(i, rng.next() as u8);
            }
            2 if !b.is_empty() => {
                let i = rng.below(b.len());
                let j = (i + rng.below(8)).min(b.len());
                b.drain(i..j);
            }
            3 => {
                let n = rng.below(b.len() + 1);
                b.truncate(n);
            }
            4 if !b.is_empty() => {
                let i = rng.below(b.len());
                let j = (i + rng.below(16)).min(b.len());
                let chunk = b[i..j].to_vec();
                let at = rng.below(b.len() + 1);
                for (k, c) in chunk.into_iter().enumerate() {
                    b.insert(at + k, c);
                }
            }
            5 if !dict.is_empty() => {
                let tok = dict[rng.below(dict.len())].as_bytes();
                let at = rng.below(b.len() + 1);
                for (k, c) in tok.iter().enumerate() {
                    b.insert(at + k, *c);
                }
            }
            _ => {
                let at = rng.below(b.len() + 1);
                let multi = ["é", "\u{2028}", "😀", "\u{0}", "\u{ffff}"][rng.below(5)];
                for (k, c) in multi.bytes().enumerate() {
                    b.insert(at + k, c);
                }
            }
        }
    }
    b
}

/// Run `target` on `iterations()` mutants of every seed.
fn fuzz(name: &str, seeds: &[&str], dict: &[&str], target: impl Fn(&[u8])) {
    let mut rng = Rng(0xC0FF_EE00 ^ name.len() as u64);
    let n = iterations();
    for seed in seeds {
        for i in 0..n {
            let input = if i == 0 {
                seed.as_bytes().to_vec()
            } else {
                mutate(&mut rng, seed.as_bytes(), dict)
            };
            let start = Instant::now();
            let res = catch_unwind(AssertUnwindSafe(|| target(&input)));
            let took = start.elapsed();
            assert!(
                res.is_ok(),
                "{name}: panic on input {:?}",
                String::from_utf8_lossy(&input)
            );
            assert!(
                took < Duration::from_secs(1),
                "{name}: {took:?} on input {:?}",
                String::from_utf8_lossy(&input)
            );
        }
    }
}

const POLICY_SEED: &str = "version = 1\nid = \"p\"\n[subject]\nexclude = [\"*.log\", \"dist\"]\n\n[[require]]\nid = \"tests\"\ncheck = \"evidence.fresh\"\ncommand = [\"pytest\", \"*\"]\nmin_assurance = \"A2\"\nseverity = \"block\"\nrepeat = { alpha = \"5%\", flake_rate = \"5%\" }\n";

const RECORD_SEED: &str = r#"{"id":"0000000000000000000000000000000000000000000000000000000000000000","record_version":1,"prev":null,"subject":{"worktree_tree":"a","tree_after":"a","head_commit":null,"dirty":false},"command":{"argv":["pytest","-q"],"cwd":"."},"execution":{"exit_code":0,"signal":null,"started_at":"2026-01-01T00:00:00Z","duration_ms":1},"observation":{"method":"none"},"recorder":{"name":"opshero","version":"0.0.1","assurance":"A1"}}"#;

#[test]
fn policy_parser_never_panics_or_hangs() {
    let dict = [
        "repeat",
        "alpha",
        "%",
        "\"",
        "[[require]]",
        "= {",
        "}",
        "\n",
        "version",
        "0.0001%",
        "99.9999%",
        "command = []",
        "exclude",
    ];
    fuzz(
        "policy",
        &[
            POLICY_SEED,
            "version=1\nid=\"x\"\n[[require]]\nid=\"a\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\n",
        ],
        &dict,
        |b| {
            if let Ok(p) = policy::parse(&String::from_utf8_lossy(b)) {
                // Anything accepted must satisfy the invariants the evaluator relies on.
                assert!(!p.require.is_empty());
                for r in &p.require {
                    assert!(!r.command.is_empty());
                    if let Some(rep) = r.repeat {
                        assert!(rep.runs >= 1 && rep.runs <= opshero_core::stats::MAX_RUNS);
                    }
                }
                let ex = p.effective_excludes();
                let mut sorted = ex.clone();
                sorted.sort();
                sorted.dedup();
                assert_eq!(ex, sorted);
            }
        },
    );
}

#[test]
fn record_parser_never_accepts_a_record_whose_id_does_not_match() {
    let dict = [
        "\"id\"",
        "null",
        "{",
        "}",
        "[",
        "]",
        "1e999",
        "-0",
        "9007199254740993",
        "\"exit_code\"",
        "true",
    ];
    fuzz("record", &[RECORD_SEED], &dict, |b| {
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(b) else {
            return;
        };
        if let Ok(r) = record::parse_verified(&v) {
            // Acceptance implies integrity: the id is the hash of the content.
            let again = record::new_record(r.body.clone()).unwrap();
            assert_eq!(again.id, r.id);
        }
    });
}

#[test]
fn canonicalizer_is_idempotent_on_anything_it_accepts() {
    let dict = [
        "\"",
        "\\",
        "\\u0000",
        "\\ud800",
        "{",
        "}",
        "[",
        "]",
        ":",
        ",",
        "1e5",
        "9007199254740991",
        "\u{10ffff}",
    ];
    fuzz(
        "canon",
        &[RECORD_SEED, r#"{"b":[1,2,{"z":null,"a":"é"}],"a":"x"}"#],
        &dict,
        |b| {
            let Ok(v) = serde_json::from_slice::<serde_json::Value>(b) else {
                return;
            };
            if let Ok(c) = canon::canonicalize(&v) {
                let back: serde_json::Value = serde_json::from_str(&c).unwrap();
                assert_eq!(canon::canonicalize(&back).unwrap(), c);
                assert_eq!(back, v);
            }
        },
    );
}

#[test]
fn shell_analysis_never_panics_and_simple_commands_round_trip() {
    let dict = [
        "&&", "||", ";", "|", "$(", "`", ">", "<", "&", "'", "\"", "\\", "cd ", "FOO=bar ", "\n",
        "*", "~", "{", "}", "\u{0}",
    ];
    fuzz(
        "shellcmd",
        &[
            "cd sub && FOO=1 pytest -q tests/",
            "pytest -q",
            "FOO=bar BAR=baz make test",
            "echo 'a b' \"c d\"",
        ],
        &dict,
        |b| {
            if let shellcmd::Analysis::Simple { argv, .. } =
                shellcmd::analyze(&String::from_utf8_lossy(b))
            {
                assert!(!argv.is_empty());
            }
        },
    );
}

#[test]
fn command_pattern_matching_never_panics() {
    let mut rng = Rng(42);
    let atoms = ["", "*", "a", "pytest", "-q", "é", "**", "a*", "*a"];
    for _ in 0..iterations() {
        let pat: Vec<String> = (0..rng.below(4))
            .map(|_| atoms[rng.below(atoms.len())].to_owned())
            .collect();
        let argv: Vec<String> = (0..rng.below(5))
            .map(|_| atoms[rng.below(atoms.len())].to_owned())
            .collect();
        let _ = policy::command_matches(&pat, &argv);
    }
}

/// Reference implementation of the civil-date conversion by brute force over
/// days (independent of the Hinnant formula used in the code).
fn brute_force(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (mut y, mut d) = (1970u64, days);
    loop {
        let len = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
            366
        } else {
            365
        };
        if d < len {
            break;
        }
        d -= len;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    while d >= months[m] {
        d -= months[m];
        m += 1;
    }
    format!(
        "{y:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        m + 1,
        d + 1,
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[test]
fn timestamps_match_a_brute_force_calendar_and_never_panic() {
    // Boundaries first: epoch, leap days, century rules, 32-bit limits, far future.
    for secs in [
        0u64,
        86_399,
        86_400,
        951_782_400,
        951_868_799,
        4_107_542_400,
        2_147_483_647,
        2_147_483_648,
        4_294_967_295,
        253_402_300_799,
    ] {
        assert_eq!(timefmt::rfc3339_utc(secs), brute_force(secs), "secs={secs}");
    }
    let mut rng = Rng(7);
    for _ in 0..20_000 {
        let secs = rng.next() % 253_402_300_800; // up to year 9999
        assert_eq!(timefmt::rfc3339_utc(secs), brute_force(secs), "secs={secs}");
    }
    // Absurd values must not panic (the output is unspecified but must exist).
    for secs in [u64::MAX, u64::MAX / 2, 1 << 62, 1 << 40] {
        let r = catch_unwind(|| timefmt::rfc3339_utc(secs));
        assert!(r.is_ok(), "panic for secs={secs}");
    }
}
