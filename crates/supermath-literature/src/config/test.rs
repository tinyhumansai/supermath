//! Tests for configuration from the environment.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::path::PathBuf;

use super::{DEFAULT_OEIS_URL, LiteratureConfig};

fn from(pairs: &[(&str, &str)]) -> LiteratureConfig {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    LiteratureConfig::from_lookup(|name| map.get(name).cloned())
}

#[test]
fn an_empty_environment_gives_the_defaults() {
    assert_eq!(from(&[]), LiteratureConfig::default());
    assert_eq!(LiteratureConfig::default().oeis_url, DEFAULT_OEIS_URL);
}

#[test]
fn variables_override_defaults() {
    let config = from(&[
        ("SUPERMATH_CONTACT", "me@example.org"),
        ("SEMANTIC_SCHOLAR_API_KEY", "k"),
        ("SUPERMATH_CACHE", "/var/cache/sm"),
        ("SUPERMATH_ARXIV_API_URL", "http://127.0.0.1:1/api"),
        ("SUPERMATH_ARXIV_EPRINT_URL", "http://127.0.0.1:1/e"),
        ("SUPERMATH_SCHOLAR_URL", "http://127.0.0.1:1/s"),
        ("SUPERMATH_OEIS_URL", "http://127.0.0.1:1/o"),
    ]);
    assert_eq!(config.contact, "me@example.org");
    assert_eq!(config.scholar_api_key.as_deref(), Some("k"));
    assert_eq!(
        config.cache_dir,
        Some(PathBuf::from("/var/cache/sm/literature"))
    );
    assert_eq!(config.arxiv_api_url, "http://127.0.0.1:1/api");
    assert_eq!(config.arxiv_eprint_url, "http://127.0.0.1:1/e");
    assert_eq!(config.scholar_url, "http://127.0.0.1:1/s");
    assert_eq!(config.oeis_url, "http://127.0.0.1:1/o");
}

#[test]
fn blank_variables_are_unset() {
    let config = from(&[("SUPERMATH_CONTACT", "  "), ("SUPERMATH_CACHE", "")]);
    assert_eq!(config.contact, "unset");
    assert_eq!(config.cache_dir, None);
}

#[test]
fn the_user_agent_names_supermath_and_the_contact() {
    let agent = from(&[("SUPERMATH_CONTACT", "me@example.org")]).user_agent();
    assert!(agent.starts_with("supermath-literature/"));
    assert!(agent.ends_with("(mailto:me@example.org)"));
}

#[test]
fn from_env_reads_the_process_environment() {
    // Whatever the environment holds, the politeness defaults are not
    // configurable through it.
    let config = LiteratureConfig::from_env();
    assert_eq!(config.arxiv_interval_ms, 3_000);
    assert_eq!(config.timeout_ms, 30_000);
}
