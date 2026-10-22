//! Tests for reading the web configuration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use super::{DEFAULT_DAILY_CAP, DEFAULT_MODEL, WebConfig};
use crate::error::Error;

fn from(pairs: &[(&str, &str)]) -> crate::Result<WebConfig> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    WebConfig::from_lookup(|name| map.get(name).cloned())
}

#[test]
fn defaults_apply_when_nothing_is_set() {
    let config = from(&[]).unwrap();
    assert!(!config.has_api_key());
    assert_eq!(config.model, DEFAULT_MODEL);
    assert_eq!(config.daily_cap, DEFAULT_DAILY_CAP);
    assert!(config.cache_dir.is_none());
    assert_eq!(
        config.user_agent(),
        format!("supermath/{}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn reads_every_variable() {
    let config = from(&[
        ("GEMINI_API_KEY", "k"),
        ("SUPERMATH_WEB_MODEL", "gemini-x"),
        ("SUPERMATH_WEB_DAILY_CAP", "7"),
        ("SUPERMATH_CACHE", "/var/cache/sm"),
        ("SUPERMATH_CONTACT", "me@example.com"),
        ("SUPERMATH_WEB_BASE_URL", "http://127.0.0.1:9/"),
    ])
    .unwrap();
    assert!(config.has_api_key());
    assert_eq!(config.model, "gemini-x");
    assert_eq!(config.daily_cap, 7);
    assert_eq!(
        config.cache_dir.as_deref(),
        Some(std::path::Path::new("/var/cache/sm/web"))
    );
    assert_eq!(config.base_url, "http://127.0.0.1:9");
    assert!(config.user_agent().contains("me@example.com"));
}

#[test]
fn empty_values_count_as_unset() {
    let config = from(&[("GEMINI_API_KEY", ""), ("SUPERMATH_WEB_MODEL", "  ")]).unwrap();
    assert!(!config.has_api_key());
    assert_eq!(config.model, DEFAULT_MODEL);
}

#[test]
fn a_bad_daily_cap_is_refused() {
    let error = from(&[("SUPERMATH_WEB_DAILY_CAP", "lots")]).unwrap_err();
    assert!(matches!(
        error,
        Error::InvalidConfig {
            variable: "SUPERMATH_WEB_DAILY_CAP",
            ..
        }
    ));
}

#[test]
fn debug_output_never_shows_the_key() {
    let config = WebConfig::default().with_api_key("sekrit-key-123");
    let shown = format!("{config:?}");
    assert!(!shown.contains("sekrit"));
    assert!(shown.contains("<redacted>"));
}

#[test]
fn builders_set_their_fields() {
    let config = WebConfig::default()
        .with_base_url("http://x/")
        .with_cache_dir("/tmp/c");
    assert_eq!(config.base_url, "http://x");
    assert!(config.cache_dir.is_some());
}
