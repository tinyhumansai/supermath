//! Tests for [`PythonConfig`](super::PythonConfig).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use super::PythonConfig;

fn config(pairs: &[(&str, &str)]) -> PythonConfig {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    PythonConfig::from_lookup(|name| map.get(name).cloned())
}

#[test]
fn defaults_apply_when_nothing_is_set() {
    let config = config(&[]);
    assert_eq!(config, PythonConfig::default());
    assert_eq!(config.url, None);
    assert_eq!(config.default_timeout_s, 60);
    assert_eq!(config.max_timeout_s, 900);
    assert_eq!(config.max_frame, 100_000_000);
}

#[test]
fn reads_every_variable_and_trims_the_url() {
    let config = config(&[
        ("PYTHON_URL", " http://python:3160/ "),
        ("SUPERMATH_PY_DEFAULT_TIMEOUT", "30"),
        ("SUPERMATH_PY_MAX_TIMEOUT", "120"),
        ("SUPERMATH_PY_MAX_FRAME", "5000"),
    ]);
    assert_eq!(config.url.as_deref(), Some("http://python:3160"));
    assert_eq!(config.default_timeout_s, 30);
    assert_eq!(config.max_timeout_s, 120);
    assert_eq!(config.max_frame, 5000);
}

#[test]
fn blank_zero_and_garbage_keep_the_defaults() {
    let config = config(&[
        ("PYTHON_URL", "  "),
        ("SUPERMATH_PY_DEFAULT_TIMEOUT", "0"),
        ("SUPERMATH_PY_MAX_TIMEOUT", "soon"),
    ]);
    assert_eq!(config, PythonConfig::default());
}

#[test]
fn the_default_timeout_never_exceeds_the_maximum() {
    let config = config(&[("SUPERMATH_PY_MAX_TIMEOUT", "20")]);
    assert_eq!(config.default_timeout_s, 20);
}

#[test]
fn from_env_reads_the_process_environment() {
    // PYTHON_URL is not set in the test environment unless a live run is
    // intended, so only the shape is asserted.
    let config = PythonConfig::from_env();
    assert!(config.max_timeout_s >= config.default_timeout_s);
}
