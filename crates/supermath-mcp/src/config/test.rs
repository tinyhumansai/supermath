//! The union of the domains' variables, read through a lookup.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use super::Config;

fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: BTreeMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn every_domain_reads_its_own_variables() {
    let config = Config::from_lookup(lookup(&[
        ("SUPERMATH_PROJECTS", "/work/projects"),
        ("LEAN_URL", "http://lean:3150"),
        ("PYTHON_URL", "http://python:3160"),
        ("SUPERMATH_CONTACT", "me@example.org"),
        ("GEMINI_API_KEY", "g-secret"),
        ("SUPERMATH_WEB_DAILY_CAP", "7"),
        ("CORTEX_API_URL", "http://cortex:3141/"),
        ("CORTEX_API_KEY", "c-secret"),
        ("CORTEX_SCOPE_ROOT", "ws:mine"),
    ]))
    .unwrap();
    assert_eq!(config.files.projects.to_str(), Some("/work/projects"));
    assert_eq!(config.lean.url.as_deref(), Some("http://lean:3150"));
    assert_eq!(config.python.url.as_deref(), Some("http://python:3160"));
    assert_eq!(config.literature.contact, "me@example.org");
    assert!(config.web.has_api_key());
    assert_eq!(config.web.daily_cap, 7);
    assert_eq!(config.ledger.cortex_url, "http://cortex:3141");
    assert_eq!(config.ledger.scope_root, "ws:mine");
    assert!(config.cortex_configured);
}

#[test]
fn an_empty_environment_leaves_every_service_unconfigured() {
    let config = Config::from_lookup(lookup(&[])).unwrap();
    assert!(config.lean.url.is_none());
    assert!(config.python.url.is_none());
    assert!(!config.web.has_api_key());
    assert!(!config.cortex_configured);
    assert_eq!(
        config.ledger.cortex_url,
        supermath_ledger::DEFAULT_CORTEX_URL
    );
}

#[test]
fn a_missing_cortex_key_still_honours_the_other_cortex_variables() {
    let config = Config::from_lookup(lookup(&[
        ("CORTEX_API_URL", "http://cortex:3141"),
        ("CORTEX_SCOPE_ROOT", "ws:mine"),
    ]))
    .unwrap();
    assert!(!config.cortex_configured);
    assert_eq!(config.ledger.cortex_url, "http://cortex:3141");
    assert_eq!(config.ledger.scope_root, "ws:mine");
}

#[test]
fn a_bad_web_cap_is_an_error() {
    let error = Config::from_lookup(lookup(&[("SUPERMATH_WEB_DAILY_CAP", "lots")])).unwrap_err();
    assert!(matches!(error, crate::Error::Web(_)), "{error}");
}

#[test]
fn the_debug_form_never_shows_a_secret() {
    let config = Config::from_lookup(lookup(&[
        ("GEMINI_API_KEY", "g-secret"),
        ("CORTEX_API_KEY", "c-secret"),
    ]))
    .unwrap();
    let shown = format!("{config:?}");
    assert!(
        !shown.contains("g-secret") && !shown.contains("c-secret"),
        "{shown}"
    );
}

#[test]
fn from_env_reads_the_process_environment() {
    // Whatever the environment holds, it composes; the CLI reads through
    // `cli::configure` instead, so this is the only caller.
    let config = super::Config::from_env().unwrap();
    assert!(!config.files.projects.as_os_str().is_empty());
}
