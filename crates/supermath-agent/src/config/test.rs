//! Unit tests for reading configuration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use super::{Config, DEFAULT_API_BASE_URL, DEFAULT_CORTEX_URL, DEFAULT_SCOPE_ROOT};
use crate::error::Error;

fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    move |name| map.get(name).cloned()
}

const REQUIRED: [(&str, &str); 3] = [
    ("SUPERMATH_API_KEY", "sk-test"),
    ("SUPERMATH_MODEL", "some/model"),
    ("CORTEX_API_KEY", "cortex-key"),
];

#[test]
fn fills_defaults_when_only_required_settings_are_given() {
    let config = Config::from_lookup(lookup(&REQUIRED)).unwrap();
    assert_eq!(config.api_base_url, DEFAULT_API_BASE_URL);
    assert_eq!(config.cortex_url, DEFAULT_CORTEX_URL);
    assert_eq!(config.scope_root, DEFAULT_SCOPE_ROOT);
    assert_eq!(config.backend_url, None);
    assert_eq!(config.model, "some/model");
}

#[test]
fn names_each_missing_required_setting() {
    for missing in ["SUPERMATH_API_KEY", "SUPERMATH_MODEL", "CORTEX_API_KEY"] {
        let pairs: Vec<_> = REQUIRED
            .iter()
            .copied()
            .filter(|(name, _)| *name != missing)
            .collect();
        let error = Config::from_lookup(lookup(&pairs)).unwrap_err();
        assert!(
            matches!(error, Error::MissingConfig(name) if name == missing),
            "{missing}: {error}"
        );
    }
}

#[test]
fn treats_a_blank_value_as_unset() {
    let mut pairs = REQUIRED.to_vec();
    pairs.retain(|(name, _)| *name != "SUPERMATH_MODEL");
    pairs.push(("SUPERMATH_MODEL", "   "));
    let error = Config::from_lookup(lookup(&pairs)).unwrap_err();
    assert!(matches!(error, Error::MissingConfig("SUPERMATH_MODEL")));
}

#[test]
fn strips_a_trailing_slash_from_the_cortex_url() {
    let mut pairs = REQUIRED.to_vec();
    pairs.push(("CORTEX_API_URL", "http://cortexdb:3141/"));
    pairs.push(("CORTEX_SCOPE_ROOT", "ws:test"));
    pairs.push(("SUPERMATH_BACKEND_URL", "http://backend"));
    let config = Config::from_lookup(lookup(&pairs)).unwrap();
    assert_eq!(config.cortex_url, "http://cortexdb:3141");
    assert_eq!(config.scope_root, "ws:test");
    assert_eq!(config.backend_url.as_deref(), Some("http://backend"));
}

#[test]
fn debug_output_never_shows_credentials() {
    let config = Config::from_lookup(lookup(&REQUIRED)).unwrap();
    let rendered = format!("{config:?}");
    assert!(!rendered.contains("sk-test"));
    assert!(!rendered.contains("cortex-key"));
    assert!(rendered.contains("some/model"));
}
