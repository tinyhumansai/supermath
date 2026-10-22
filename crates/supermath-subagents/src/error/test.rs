//! Tests for the error messages.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Error;

#[test]
fn messages_are_lowercase_and_name_the_subject() {
    let invalid = Error::InvalidConfig {
        name: "SUPERMATH_READER_MODEL",
        value: String::new(),
    };
    assert_eq!(
        invalid.to_string(),
        "setting `SUPERMATH_READER_MODEL` has an invalid value ``"
    );
    let agent = Error::Agent {
        id: "supermath-x-formalizer".into(),
        detail: "too long".into(),
    };
    assert!(agent.to_string().contains("supermath-x-formalizer"));
    assert_eq!(
        Error::NotPrimitive("web_research".into()).to_string(),
        "`web_research` is an agentic tool and cannot be a child's primitive"
    );
    assert_eq!(
        Error::Bench("eof".into()).to_string(),
        "benchmark set is malformed: eof"
    );
    assert_eq!(
        Error::MissingPrimitive("lean_check".into()).to_string(),
        "tool registry lacks `lean_check`"
    );
}

#[test]
fn a_registry_error_converts_transparently() {
    let inner = supermath_tool::Error::DuplicateTool("lean_check".into());
    let message = inner.to_string();
    let outer: Error = inner.into();
    assert_eq!(outer.to_string(), message);
}
