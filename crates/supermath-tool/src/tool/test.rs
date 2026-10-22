//! Pins the serde forms of the tool descriptors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Backend, Exposure, Kind, Permission};

#[test]
fn exposure_and_permission_are_snake_case_strings() {
    assert_eq!(
        serde_json::to_value(Exposure::Direct).unwrap(),
        json!("direct")
    );
    assert_eq!(
        serde_json::to_value(Exposure::Deferred).unwrap(),
        json!("deferred")
    );
    assert_eq!(
        serde_json::to_value(Permission::Read).unwrap(),
        json!("read")
    );
    assert_eq!(
        serde_json::to_value(Permission::Write).unwrap(),
        json!("write")
    );
}

#[test]
fn kind_serialises_primitive_and_agentic_backends() {
    assert_eq!(
        serde_json::to_value(Kind::Primitive).unwrap(),
        json!("primitive")
    );
    assert_eq!(
        serde_json::to_value(Kind::Agentic(Backend::Gemini)).unwrap(),
        json!({"agentic": "gemini"})
    );
    assert_eq!(
        serde_json::to_value(Kind::Agentic(Backend::OpenHuman)).unwrap(),
        json!({"agentic": "openhuman"})
    );
    let back: Kind = serde_json::from_value(json!({"agentic": "openhuman"})).unwrap();
    assert_eq!(back, Kind::Agentic(Backend::OpenHuman));
}

#[test]
fn only_agentic_kinds_are_agentic() {
    assert!(!Kind::Primitive.is_agentic());
    assert!(Kind::Agentic(Backend::Gemini).is_agentic());
}
