//! Unit tests for [`LeanConfig`].

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::{DEFAULT_LEANSEARCH_URL, DEFAULT_LOOGLE_URL, LeanConfig};

#[test]
fn nothing_set_means_no_service_and_default_indexes() {
    let config = LeanConfig::from_lookup(|_| None);
    assert_eq!(config.url, None);
    assert_eq!(config.loogle_url, DEFAULT_LOOGLE_URL);
    assert_eq!(config.leansearch_url, DEFAULT_LEANSEARCH_URL);
    assert_eq!(config, LeanConfig::default());
}

#[test]
fn hosted_searches_get_thirty_seconds() {
    // Loogle took 15 s to answer `_ + 0 = _` live, so 15 s timed it out.
    assert_eq!(LeanConfig::default().query_timeout_ms, 30_000);
}

#[test]
fn variables_override_and_blank_ones_do_not() {
    let config = LeanConfig::from_lookup(|name| match name {
        "LEAN_URL" => Some(" http://lean:3150/ ".to_owned()),
        "LOOGLE_URL" => Some("http://loogle.test/json".to_owned()),
        "LEANSEARCH_URL" => Some("   ".to_owned()),
        _ => None,
    });
    assert_eq!(config.url.as_deref(), Some("http://lean:3150"));
    assert_eq!(config.loogle_url, "http://loogle.test/json");
    assert_eq!(config.leansearch_url, DEFAULT_LEANSEARCH_URL);
}

#[test]
fn from_env_reads_the_process_environment() {
    // Only checks that it runs; the environment is the test runner's.
    let _ = LeanConfig::from_env();
}
