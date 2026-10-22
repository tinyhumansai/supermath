//! Unit tests for the runner's `OpenHuman` configuration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Runner, fixed_config, jev_credential, mcp_servers, runtime_config, stdio_server};
use crate::config::Config;

fn config() -> Config {
    Config::from_lookup(|name| match name {
        "LADDER_API_KEY" => Some("ladder-key".into()),
        "OPENROUTER_API_KEY" => Some("sk-or-test".into()),
        "CORTEX_API_KEY" => Some("cortex-key".into()),
        "SUPERMATH_REASONING_EFFORT" => Some("high".into()),
        _ => None,
    })
    .unwrap()
}

#[test]
fn runtime_config_routes_tool_search_through_jev_on_openrouter() {
    let runtime = runtime_config(&config());
    assert_eq!(runtime.agent.tool_search.ranker, "jev");
    assert_eq!(runtime.agent.tool_search.jev_route, "openrouter");
    assert_eq!(runtime.runtime.reasoning_effort.as_deref(), Some("high"));
    assert!(!runtime.local_ai.runtime_enabled);
}

#[tokio::test]
async fn the_ranker_reads_the_runner_config_and_only_the_openrouter_key() {
    let runtime = runtime_config(&config());
    let loaded = fixed_config(runtime)().await.unwrap();
    assert_eq!(loaded.agent.tool_search.jev_route, "openrouter");

    let env = jev_credential("sk-or-test".into());
    assert_eq!(env("OPENROUTER_API_KEY").as_deref(), Some("sk-or-test"));
    assert_eq!(env("TYPESAFE_API_KEY"), None);
    assert_eq!(env("PATH"), None);
}

#[test]
fn a_command_line_becomes_a_stdio_server_per_configured_command() {
    let mut settings = config();
    assert!(mcp_servers(&settings).is_empty());

    settings.jacobian_cmd = Some("jacobian-mcp --stdio".into());
    settings.lean_lsp_cmd = Some("uvx lean-lsp-mcp".into());
    assert_eq!(mcp_servers(&settings).len(), 2);

    settings.jacobian_cmd = None;
    assert_eq!(mcp_servers(&settings).len(), 1);
}

#[test]
fn a_blank_command_line_is_no_server() {
    assert!(stdio_server("jacobian", "   ").is_none());
    assert!(stdio_server("jacobian", "jacobian-mcp").is_some());
}

#[test]
fn the_agent_scope_lists_the_toolkit_the_agent_tools_and_tool_search_and_refuses_an_unnameable_project()
 {
    let home = tempfile::tempdir().unwrap();
    let settings = Config::from_lookup(|name| match name {
        "LADDER_API_KEY" | "OPENROUTER_API_KEY" | "CORTEX_API_KEY" => Some("k".into()),
        "SUPERMATH_WORKSPACE" => Some(home.path().join("workspace").display().to_string()),
        "SUPERMATH_PROJECTS" => Some(home.path().join("projects").display().to_string()),
        "SUPERMATH_JACOBIAN_CMD" => Some("jacobian-mcp --stdio".into()),
        _ => None,
    })
    .unwrap();
    // One runtime per process, so both checks share it.
    let (solver, refused) = on_agent_stack(async move {
        let runner = Runner::start(&settings).await.unwrap();
        (
            runner.agent("Twin Primes").unwrap(),
            runner.agent("!!").unwrap_err(),
        )
    });
    assert!(matches!(refused, crate::Error::Toolkit(_)), "{refused}");

    assert_eq!(solver.agent().id(), "supermath-twin-primes");
    assert_eq!(solver.unavailable(), [] as [String; 0]);
    assert_eq!(
        solver.scope(),
        [
            "calc_factor",
            "calc_gcd_lcm",
            "file_edit",
            "file_list",
            "file_read",
            "file_write",
            "lean_blueprint",
            "lean_build",
            "lean_check",
            "lean_decl",
            "lean_goals",
            "lean_search",
            "lean_search_counterexample",
            "lean_try_tactics",
            "ledger_bound",
            "ledger_claims",
            "ledger_link",
            "ledger_record",
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
            "agent_digest_paper",
            "agent_formalize",
            "agent_refute",
            "agent_review_alignment",
            "tool_search",
        ]
    );
    assert!(!solver.scope().iter().any(|name| name == "memory"));
    assert_eq!(solver.host().tool_names().len() + 1, solver.scope().len());
}

/// Runs `future` on a task of `OpenHuman`'s runtime, whose worker stacks fit
/// agent turns; a test thread's default stack does not.
fn on_agent_stack<T: Send + 'static>(
    future: impl std::future::Future<Output = T> + Send + 'static,
) -> T {
    let runtime = openhuman_embed::process::tokio_runtime().unwrap();
    runtime.block_on(async { tokio::spawn(future).await.unwrap() })
}
