//! Composition without any backend configured: the catalogue, the project,
//! and the call log.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use supermath_tool::contract::description_violation;
use supermath_tool::{ErrorCode, Exposure, Outcome};

use super::Toolkit;
use crate::{Config, Error};

/// Every non-`agent_*` tool of the catalogue in `docs/specs/toolkit/README.md`.
const CATALOGUE: [&str; 30] = [
    "calc_factor",
    "calc_gcd_lcm",
    "file_edit",
    "file_list",
    "file_read",
    "file_write",
    "ledger_bound",
    "ledger_claims",
    "ledger_link",
    "ledger_record",
    "lean_blueprint",
    "lean_build",
    "lean_check",
    "lean_decl",
    "lean_goals",
    "lean_search",
    "lean_search_counterexample",
    "lean_try_tactics",
    "lit_citations",
    "lit_extract_statements",
    "lit_get_paper",
    "lit_oeis",
    "lit_search_arxiv",
    "py_run",
    "py_search_counterexample",
    "py_solvers",
    "recall_memory",
    "web_read",
    "web_research",
    "web_search",
];

/// The tools always in the model's catalogue; every other tool is deferred
/// (`docs/specs/toolkit/README.md`, "Progressive disclosure").
const DIRECT: [&str; 6] = [
    "lean_check",
    "ledger_record",
    "lit_search_arxiv",
    "py_run",
    "recall_memory",
    "web_search",
];

fn toolkit() -> Toolkit {
    Toolkit::compose(&Config::from_lookup(|_| None).unwrap(), "Twin Primes").unwrap()
}

#[test]
fn the_registry_holds_the_catalogue_and_no_agent_tool() {
    let registry = toolkit().registry();
    let mut names: Vec<&str> = registry.names().collect();
    names.sort_unstable();
    let mut expected = CATALOGUE.to_vec();
    expected.sort_unstable();
    assert_eq!(names.len(), CATALOGUE.len());
    for name in expected {
        assert!(registry.get(name).is_some(), "missing {name}");
    }
    assert!(names.iter().all(|n| !n.starts_with("agent_")), "{names:?}");
}

#[test]
fn the_project_is_the_slug_of_the_name_given() {
    let toolkit = toolkit();
    assert_eq!(toolkit.project().slug(), "twin-primes");
    assert_eq!(
        toolkit.project().scope(),
        "ws:supermath/project:twin-primes"
    );
}

#[test]
fn a_name_with_nothing_usable_is_refused() {
    let config = Config::from_lookup(|_| None).unwrap();
    let error = Toolkit::compose(&config, "***").unwrap_err();
    assert!(matches!(error, Error::InvalidProject(_)), "{error}");
}

#[tokio::test]
async fn dispatch_numbers_calls_and_logs_them() {
    let toolkit = toolkit();
    let first = toolkit.dispatch("calc_factor", json!({"n": 12})).await;
    let second = toolkit
        .dispatch("calc_gcd_lcm", json!({"a": 4, "b": 6}))
        .await;
    assert_eq!(
        (first.call_id.as_str(), second.call_id.as_str()),
        ("call-1", "call-2")
    );
    assert_eq!(first.envelope.outcome, Outcome::Ok);
    let log = toolkit.call_log();
    let record = log.get("call-1").unwrap();
    assert_eq!(record.tool, "calc_factor");
    assert_eq!(record.project, "twin-primes");
    assert_eq!(log.all().len(), 2);
}

#[tokio::test]
async fn an_unknown_tool_is_not_found_and_still_logged() {
    let toolkit = toolkit();
    let done = toolkit.dispatch("agent_formalize", json!({})).await;
    assert_eq!(done.envelope.code(), Some(ErrorCode::NotFound));
    assert_eq!(toolkit.call_log().all().len(), 1);
}

#[tokio::test]
async fn a_caller_built_context_is_used_as_given() {
    let toolkit = toolkit();
    let ctx = toolkit.context("mine-1");
    let envelope = toolkit
        .dispatch_with(&ctx, "calc_factor", json!({"n": 7}))
        .await;
    assert!(envelope.is_ok());
    assert!(toolkit.call_log().get("mine-1").is_some());
}

#[test]
fn the_domain_handles_are_reachable_and_gemini_is_a_presence_only() {
    let toolkit = toolkit();
    assert!(!toolkit.has_gemini_key());
    assert!(!toolkit.lean().client().is_configured());
    let _ = (toolkit.python(), toolkit.literature(), toolkit.memory());
}

#[test]
fn exactly_the_six_direct_tools_are_direct_and_every_description_names_its_question() {
    let registry = toolkit().registry();
    let mut direct: Vec<&str> = registry
        .iter()
        .filter(|tool| tool.exposure() == Exposure::Direct)
        .map(|tool| tool.name())
        .collect();
    direct.sort_unstable();
    assert_eq!(direct, DIRECT);
    for tool in registry.iter() {
        assert_eq!(description_violation(tool.as_ref()), None, "{}", tool.name());
    }
}
