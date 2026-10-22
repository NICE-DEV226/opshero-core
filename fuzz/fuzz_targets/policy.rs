//! Coverage-guided fuzzing of the policy parser. Properties, not just "does not crash": whatever
//! is accepted must satisfy the invariants the evaluator relies on, and parsing is deterministic.
#![no_main]
use libfuzzer_sys::fuzz_target;
use opshero_core::policy;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let first = policy::parse(&text);
    let second = policy::parse(&text);
    assert_eq!(first.is_ok(), second.is_ok(), "parsing must be deterministic");
    if let Ok(p) = first {
        assert!(!p.require.is_empty());
        for r in &p.require {
            assert!(!r.command.is_empty());
            if let Some(rep) = r.repeat {
                assert!(rep.runs >= 1 && rep.runs <= opshero_core::stats::MAX_RUNS);
            }
            if let Some(run) = &r.run {
                assert!(policy::command_matches(&r.command, run), "`run` must match `command`");
            }
        }
        let ex = p.effective_excludes();
        let mut sorted = ex.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ex, sorted, "effective exclusions must be sorted and de-duplicated");
        for e in &ex {
            assert!(!e.chars().all(|c| matches!(c, '*' | '.' | '/')), "no exclusion may match everything");
        }
    }
});
