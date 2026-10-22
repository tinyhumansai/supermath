//! Configuration parsing, through a lookup function.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Config, DEFAULT_CORTEX_URL, DEFAULT_SCOPE_ROOT};
use crate::error::Error;

fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| (*v).to_string())
    }
}

#[test]
fn defaults_apply_when_only_the_key_is_set() {
    let config = Config::from_lookup(lookup(&[("CORTEX_API_KEY", "k")])).unwrap();
    assert_eq!(config.cortex_url, DEFAULT_CORTEX_URL);
    assert_eq!(config.scope_root, DEFAULT_SCOPE_ROOT);
    assert_eq!(config.cortex_api_key, "k");
}

#[test]
fn every_variable_is_read_and_the_url_loses_its_trailing_slash() {
    let config = Config::from_lookup(lookup(&[
        ("CORTEX_API_KEY", " key "),
        ("CORTEX_API_URL", "http://cortexdb:3141/"),
        ("CORTEX_SCOPE_ROOT", "ws:test"),
    ]))
    .unwrap();
    assert_eq!(config.cortex_url, "http://cortexdb:3141");
    assert_eq!(config.cortex_api_key, "key");
    assert_eq!(
        config
            .memory()
            .scope(&crate::memory::Project::new("p").unwrap()),
        "ws:test/project:p/app:brain"
    );
}

#[test]
fn a_missing_or_blank_key_is_refused() {
    for pairs in [&[][..], &[("CORTEX_API_KEY", "  ")][..]] {
        let error = Config::from_lookup(lookup(pairs)).unwrap_err();
        assert!(matches!(error, Error::MissingConfig("CORTEX_API_KEY")));
    }
}

#[test]
fn debug_output_never_carries_the_key() {
    let config = Config::from_lookup(lookup(&[("CORTEX_API_KEY", "secret-value")])).unwrap();
    assert!(!format!("{config:?}").contains("secret-value"));
}

#[test]
fn from_env_reads_the_process_environment() {
    // Only the error path is asserted: the test environment sets no key.
    if std::env::var("CORTEX_API_KEY").is_err() {
        assert!(Config::from_env().is_err());
    } else {
        assert!(Config::from_env().is_ok());
    }
}
