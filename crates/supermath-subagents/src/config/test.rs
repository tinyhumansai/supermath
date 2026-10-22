//! Tests for reading the model names.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Models;
use crate::error::Error;

#[test]
fn defaults_are_the_documented_surplus_models() {
    let models = Models::from_lookup(|_| None).unwrap();
    assert_eq!(models, Models::default());
    assert_eq!(models.reader, "deepseek-v4.1-flash");
    assert_eq!(models.formalizer, "deepseek-v4-pro");
    assert_eq!(models.reviewer, "deepseek-v4-pro");
}

#[test]
fn each_variable_overrides_only_its_own_model() {
    let models = Models::from_lookup(|name| match name {
        "SUPERMATH_FORMALIZER_MODEL" => Some("kimi-k3".into()),
        "SUPERMATH_REVIEWER_MODEL" => Some("glm-5.3".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(models.reader, "deepseek-v4.1-flash");
    assert_eq!(models.formalizer, "kimi-k3");
    assert_eq!(models.reviewer, "glm-5.3");
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
