//! Canonical JSON: whatever is accepted round-trips and is idempotent.
#![no_main]
use libfuzzer_sys::fuzz_target;
use opshero_core::canon::canonicalize;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) else {
        return;
    };
    if let Ok(c) = canonicalize(&value) {
        let back: serde_json::Value = serde_json::from_str(&c).expect("canonical text is valid JSON");
        assert_eq!(back, value, "canonicalization must not change the value");
        assert_eq!(canonicalize(&back).expect("accepted twice"), c, "canonicalization must be idempotent");
    }
});
