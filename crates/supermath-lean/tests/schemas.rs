//! The tools' schemas, as the model sees them.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use std::collections::BTreeSet;

use serde_json::Value;
use supermath_tool::{Exposure, Permission};
use support::{Env, config};

/// Every property name anywhere in a schema.
fn property_names(schema: &Value, found: &mut BTreeSet<String>) {
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, inner) in properties {
            found.insert(name.clone());
            property_names(inner, found);
        }
    }
    if let Some(items) = schema.get("items") {
        property_names(items, found);
    }
}

#[test]
fn no_lean_tool_takes_lean_source_as_an_argument() {
    // The only string arguments are paths, declaration names, queries, short
    // tactic lines, and digests; there is no `source`, `code`, or `content`.
    let allowed: BTreeSet<&str> = [
        "path",
        "targets",
        "query",
        "mode",
        "limit",
        "name",
        "line",
        "column",
        "sorry",
        "tactics",
        "if_sha256",
        "predicate",
        "range",
        "from",
        "to",
        "root",
        "cursor",
    ]
    .into_iter()
    .collect();
    let env = Env::new(config(None));
    for tool in env.lean.tools() {
        let mut found = BTreeSet::new();
        property_names(&tool.schema(), &mut found);
        for name in &found {
            assert!(
                allowed.contains(name.as_str()),
                "{} has an unexpected argument {name:?}",
                tool.name()
            );
        }
        assert_eq!(
            tool.schema()["additionalProperties"],
            false,
            "{}",
            tool.name()
        );
    }
}

#[test]
fn the_catalogue_matches_the_spec() {
    let env = Env::new(config(None));
    let tools = env.lean.tools();
    let table: Vec<_> = tools
        .iter()
        .map(|t| {
            (
                t.name(),
                t.exposure() == Exposure::Direct,
                t.permission() == Permission::Read,
            )
        })
        .collect();
    assert_eq!(
        table,
        [
            ("lean_check", true, true),
            ("lean_search", false, true),
            ("lean_decl", false, true),
            ("lean_goals", false, true),
            ("lean_try_tactics", false, true),
            ("lean_search_counterexample", false, true),
            ("lean_build", false, true),
            ("lean_blueprint", false, true),
        ]
    );
    for tool in &tools {
        assert!(!tool.kind().is_agentic(), "{}", tool.name());
        assert!(!tool.description().is_empty());
    }
}
