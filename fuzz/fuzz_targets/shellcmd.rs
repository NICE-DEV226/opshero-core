//! The shell command analyzer: it must never panic, be deterministic, and only call something a simple
//! command if its program name looks like a program name and the input has no unusual whitespace.
#![no_main]
use libfuzzer_sys::fuzz_target;
use opshero_core::shellcmd::{Analysis, analyze};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let a = analyze(&text);
    assert_eq!(a, analyze(&text), "analysis must be deterministic");
    if let Analysis::Simple { argv, env_names, .. } = a {
        assert!(!argv.is_empty());
        assert!(!argv[0].is_empty());
        assert!(
            argv[0].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '+' | '-' | ':' | '@' | '%')),
            "program name {:?}",
            argv[0]
        );
        assert!(
            !text.chars().any(|c| (c.is_control() && c != '\t' && c != '\n') || (c.is_whitespace() && !matches!(c, ' ' | '\t' | '\n'))),
            "a simple command must not contain control characters or exotic whitespace"
        );
        for n in env_names {
            assert!(!n.is_empty());
        }
    }
});
