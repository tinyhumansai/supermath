//! Unit tests for reading configuration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use super::{
    Config, DEFAULT_EMBEDDING_DIMS, DEFAULT_EMBEDDING_MODEL, DEFAULT_SURPLUS_URL, DEFAULT_MODEL,
    DEFAULT_REASONING_EFFORT,
};
use supermath_subagents::Models;

use crate::error::Error;

fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    move |name| map.get(name).cloned()
}

const REQUIRED: [(&str, &str); 3] = [
    ("SURPLUS_API_KEY", "surplus-key"),
    ("OPENROUTER_API_KEY", "sk-or-test"),
    ("CORTEX_API_KEY", "cortex-key"),
];

fn with(extra: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
    let mut pairs = REQUIRED.to_vec();
    pairs.retain(|(name, _)| !extra.iter().any(|(e, _)| e == name));
    pairs.extend_from_slice(extra);
    pairs
}

#[test]
fn fills_defaults_when_only_required_settings_are_given() {
    let config = Config::from_lookup(lookup(&REQUIRED)).unwrap();
    assert_eq!(config.surplus_url, DEFAULT_SURPLUS_URL);
    assert_eq!(config.model, DEFAULT_MODEL);
    assert_eq!(config.reasoning_effort, DEFAULT_REASONING_EFFORT);
    assert_eq!(config.embedding_model, DEFAULT_EMBEDDING_MODEL);
    assert_eq!(config.embedding_dims, DEFAULT_EMBEDDING_DIMS);
    assert_eq!(config.toolkit.ledger.cortex_url, "http://localhost:3141");
    assert_eq!(config.toolkit.ledger.scope_root, "ws:supermath");
    assert_eq!(config.models, Models::default());
    assert_eq!(config.jacobian_cmd, None);
    assert_eq!(config.lean_lsp_cmd, None);
    assert_eq!(config.backend_url, None);
    assert_eq!(config.openrouter_api_key, "sk-or-test");
}

#[test]
fn names_each_missing_required_setting() {
    for (missing, _) in REQUIRED {
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
    let error = Config::from_lookup(lookup(&with(&[("SURPLUS_API_KEY", "   ")]))).unwrap_err();
    assert!(matches!(error, Error::MissingConfig("SURPLUS_API_KEY")));
}

#[test]
fn reads_overrides_and_strips_trailing_slashes() {
    let config = Config::from_lookup(lookup(&with(&[
        ("SURPLUS_URL", "http://ladder:6969/v1/"),
        ("CORTEX_API_URL", "http://cortexdb:3141/"),
        ("CORTEX_SCOPE_ROOT", "ws:test"),
        ("SUPERMATH_BACKEND_URL", "http://backend"),
        ("SUPERMATH_MODEL", "reasoning"),
        ("SUPERMATH_REASONING_EFFORT", "high"),
        ("SUPERMATH_EMBEDDING_DIMS", "1536"),
        ("SUPERMATH_READER_MODEL", "cheap"),
        ("SUPERMATH_JACOBIAN_CMD", " jacobian-mcp --stdio "),
        ("SUPERMATH_LEAN_LSP_CMD", "uvx lean-lsp-mcp"),
    ])))
    .unwrap();
    assert_eq!(config.surplus_url, "http://ladder:6969/v1");
    assert_eq!(config.toolkit.ledger.cortex_url, "http://cortexdb:3141");
    assert_eq!(config.toolkit.ledger.scope_root, "ws:test");
    assert_eq!(config.backend_url.as_deref(), Some("http://backend"));
    assert_eq!(config.model, "reasoning");
    assert_eq!(config.reasoning_effort, "high");
    assert_eq!(config.embedding_dims, 1536);
    assert_eq!(config.models.reader, "cheap");
    assert_eq!(config.jacobian_cmd.as_deref(), Some("jacobian-mcp --stdio"));
    assert_eq!(config.lean_lsp_cmd.as_deref(), Some("uvx lean-lsp-mcp"));
}

#[test]
fn rejects_a_child_model_that_is_not_a_ladder_name() {
    let error =
        Config::from_lookup(lookup(&with(&[("SUPERMATH_READER_MODEL", "two words")]))).unwrap_err();
    assert!(matches!(error, Error::Subagents(_)), "{error}");
}

#[test]
fn rejects_an_invalid_toolkit_setting() {
    let error =
        Config::from_lookup(lookup(&with(&[("SUPERMATH_WEB_DAILY_CAP", "lots")]))).unwrap_err();
    assert!(matches!(error, Error::Toolkit(_)), "{error}");
}

#[test]
fn rejects_embedding_dims_that_are_not_a_positive_integer() {
    for raw in ["0", "-1", "wide"] {
        let error =
            Config::from_lookup(lookup(&with(&[("SUPERMATH_EMBEDDING_DIMS", raw)]))).unwrap_err();
        assert!(
            matches!(&error, Error::InvalidConfig { name: "SUPERMATH_EMBEDDING_DIMS", value } if value == raw),
            "{raw}: {error}"
        );
    }
}

#[test]
fn rejects_a_reasoning_effort_openhuman_does_not_accept() {
    let error = Config::from_lookup(lookup(&with(&[("SUPERMATH_REASONING_EFFORT", "extreme")])))
        .unwrap_err();
    assert!(
        matches!(&error, Error::InvalidConfig { name: "SUPERMATH_REASONING_EFFORT", value } if value == "extreme"),
        "{error}"
    );
}

#[test]
fn debug_output_never_shows_credentials() {
    let config = Config::from_lookup(lookup(&REQUIRED)).unwrap();
    let rendered = format!("{config:?}");
    for secret in ["surplus-key", "sk-or-test", "cortex-key"] {
        assert!(!rendered.contains(secret), "{secret}");
    }
    assert!(rendered.contains(DEFAULT_MODEL));
}
