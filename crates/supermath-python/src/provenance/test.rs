//! Tests for canonical JSON and the provenance wire form.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Provenance, args_sha256, canonical_json};

#[test]
fn canonical_json_sorts_keys_at_every_depth() {
    let value = json!({"z": {"b": 1, "a": [true, null, 2.5]}, "a": "q\"x"});
    assert_eq!(
        canonical_json(&value),
        r#"{"a":"q\"x","z":{"a":[true,null,2.5],"b":1}}"#
    );
}

#[test]
fn key_order_does_not_change_the_digest() {
    let left = json!({"n": 6, "k": 3});
    let right: serde_json::Value = serde_json::from_str(r#"{"k":3,"n":6}"#).unwrap();
    assert_eq!(args_sha256(&left), args_sha256(&right));
    assert_ne!(args_sha256(&left), args_sha256(&json!({"n": 6, "k": 4})));
}

#[test]
fn the_digest_is_pinned_for_empty_arguments() {
    // SHA-256 of "{}".
    assert_eq!(
        args_sha256(&json!({})),
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
}

#[test]
fn provenance_pins_its_wire_form() {
    let provenance = Provenance {
        path: "python/work/a.py".into(),
        file_sha256: "f".into(),
        imports_sha256: "i".into(),
        entry: "go".into(),
        args_sha256: "a".into(),
        seed: 3,
        python: "3.12.1".into(),
        lock_sha256: Some("l".into()),
        elapsed_ms: 12,
        peak_rss_mb: None,
    };
    assert_eq!(
        serde_json::to_value(&provenance).unwrap(),
        json!({
            "path": "python/work/a.py", "file_sha256": "f", "imports_sha256": "i",
            "entry": "go", "args_sha256": "a", "seed": 3, "python": "3.12.1",
            "lock_sha256": "l", "elapsed_ms": 12, "peak_rss_mb": null
        })
    );
}
