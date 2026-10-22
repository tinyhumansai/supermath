//! Tests for reading the model names.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Models;
use crate::error::Error;

#[test]
fn defaults_are_the_documented_rungs() {
    let models = Models::from_lookup(|_| None).unwrap();
    assert_eq!(models, Models::default());
    assert_eq!(models.reader, "flash");
    assert_eq!(models.formalizer, "formalize");
    assert_eq!(models.reviewer, "reasoning");
}

#[test]
fn each_variable_overrides_only_its_own_rung() {
    let models = Models::from_lookup(|name| match name {
        "SUPERMATH_FORMALIZER_MODEL" => Some("kimina".into()),
        "SUPERMATH_REVIEWER_MODEL" => Some("deep".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(models.reader, "flash");
    assert_eq!(models.formalizer, "kimina");
    assert_eq!(models.reviewer, "deep");
}

#[test]
fn a_blank_or_spaced_value_is_refused_by_name() {
    for bad in ["", "two words"] {
        let error =
            Models::from_lookup(|name| (name == "SUPERMATH_READER_MODEL").then(|| bad.to_string()))
                .unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidConfig {
                name: "SUPERMATH_READER_MODEL",
                ..
            }
        ));
    }
}

#[test]
fn from_env_reads_the_process_environment_without_failing_on_unset_names() {
    // The three variables are not set by the test harness; whatever is set in
    // the developer's shell must still parse or be refused cleanly.
    let _ = Models::from_env();
}
