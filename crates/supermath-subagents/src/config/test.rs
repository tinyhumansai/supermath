//! Tests for reading the model names.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{DuelSettings, Models};
use crate::error::Error;

#[test]
fn defaults_are_the_documented_surplus_models() {
    let models = Models::from_lookup(|_| None).unwrap();
    assert_eq!(models, Models::default());
    assert_eq!(models.reader, "deepseek-v4.1-flash");
    assert_eq!(models.formalizer, "deepseek-v4-pro");
    assert_eq!(models.reviewer, "glm-5.3");
    assert_eq!(models.duel, "deepseek-v4-pro");
}

#[test]
fn each_variable_overrides_only_its_own_model() {
    let models = Models::from_lookup(|name| match name {
        "SUPERMATH_FORMALIZER_MODEL" => Some("kimi-k3".into()),
        "SUPERMATH_REVIEWER_MODEL" => Some("kimi-k2.6".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(models.reader, "deepseek-v4.1-flash");
    assert_eq!(models.formalizer, "kimi-k3");
    assert_eq!(models.reviewer, "kimi-k2.6");
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

#[test]
fn the_duel_model_is_its_own_variable() {
    let models =
        Models::from_lookup(|name| (name == "SUPERMATH_DUEL_MODEL").then(|| "kimi-k3".into()))
            .unwrap();
    assert_eq!(models.duel, "kimi-k3");
    assert_eq!(models.formalizer, "deepseek-v4-pro");
}

#[test]
fn duel_settings_default_to_the_documented_values() {
    let settings = DuelSettings::from_lookup(|_| None).unwrap();
    assert_eq!(settings, DuelSettings::default());
    assert_eq!(
        (
            settings.drift_calls,
            settings.max_rounds,
            settings.digest_every
        ),
        (12, 4, 8)
    );
}

#[test]
fn each_duel_variable_overrides_only_its_own_setting() {
    let settings = DuelSettings::from_lookup(|name| match name {
        "SUPERMATH_DUEL_DRIFT_CALLS" => Some("20".into()),
        "SUPERMATH_DUEL_DIGEST_EVERY" => Some(" 3 ".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(
        (
            settings.drift_calls,
            settings.max_rounds,
            settings.digest_every
        ),
        (20, 4, 3)
    );
}

#[test]
fn a_duel_setting_that_is_zero_or_not_a_number_is_refused_by_name() {
    for bad in ["0", "-1", "many", ""] {
        let error = DuelSettings::from_lookup(|name| {
            (name == "SUPERMATH_DUEL_MAX_ROUNDS").then(|| bad.to_string())
        })
        .unwrap_err();
        assert!(
            matches!(
                error,
                Error::InvalidConfig {
                    name: "SUPERMATH_DUEL_MAX_ROUNDS",
                    ..
                }
            ),
            "{bad}"
        );
    }
    let _ = DuelSettings::from_env();
}

#[test]
fn a_reviewer_on_the_formalizers_model_is_refused() {
    let error = Models::from_lookup(|name| {
        (name == "SUPERMATH_REVIEWER_MODEL").then(|| "deepseek-v4-pro".into())
    })
    .unwrap_err();
    assert!(
        matches!(&error, Error::ReviewerIsFormalizer(model) if model == "deepseek-v4-pro"),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        "the reviewer must not run on the formalizer's model `deepseek-v4-pro`"
    );
    let same = Models {
        reviewer: "deepseek-v4-pro".into(),
        ..Models::default()
    };
    assert!(matches!(same.check(), Err(Error::ReviewerIsFormalizer(_))));
    assert!(Models::default().check().is_ok());
}
