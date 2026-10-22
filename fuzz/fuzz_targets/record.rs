//! A record is accepted only if its id is the hash of its content (the integrity check).
#![no_main]
use libfuzzer_sys::fuzz_target;
use opshero_core::record;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) else {
        return;
    };
    if let Ok(rec) = record::parse_verified(&value) {
        // A record may carry unknown fields: they are covered by the id (tolerated, but hashed), and
        // re-hashing only the known ones would give another id. The property is therefore checked
        // only when nothing was dropped on the way in.
        let reserialized = serde_json::to_value(&rec).expect("a record serializes");
        if reserialized == value {
            let again = record::new_record(rec.body.clone()).expect("an accepted record can be re-hashed");
            assert_eq!(again.id, rec.id, "acceptance must imply that the id is the hash of the content");
        }
    }
});
