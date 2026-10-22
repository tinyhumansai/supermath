//! The acceptance criteria of `docs/specs/toolkit/mcp/README.md`, over MCP,
//! with a real client on the other end of an in-memory pipe.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult, Tool};
use rmcp::service::{RoleClient, RunningService};
use serde_json::{Value, json};
use supermath_mcp::{Toolkit, serve_on};
use support::{CLEAN_LEAN, Env, LEAN_PATH, LeanService, cortex};

type Client = RunningService<RoleClient, ()>;

async fn connect(toolkit: Toolkit) -> Client {
    let (client_io, server_io) = tokio::io::duplex(1 << 20);
    tokio::spawn(serve_on(Arc::new(toolkit), server_io));
    ().serve(client_io).await.unwrap()
}

async fn call(client: &Client, name: &str, args: Value) -> (CallToolResult, Value) {
    let arguments = args.as_object().cloned().unwrap_or_default();
    let result = client
        .call_tool(CallToolRequestParams::new(name.to_owned()).with_arguments(arguments))
        .await
        .unwrap();
    let envelope = serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap();
    (result, envelope)
}

fn names(tools: &[Tool]) -> Vec<String> {
    let mut names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    names.sort();
    names
}

#[tokio::test]
async fn tools_list_has_every_primitive_and_no_agent_tool() {
    let client = connect(Env::new().toolkit()).await;
    let tools = client.list_all_tools().await.unwrap();
    let listed = names(&tools);
    for expected in [
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
        "recall_memory",
        "lean_blueprint",
        "lean_build",
        "lean_check",
        "lean_decl",
        "lean_goals",
        "lean_search",
        "lean_search_counterexample",
        "lean_try_tactics",
        "py_run",
        "py_search_counterexample",
        "py_solvers",
        "lit_citations",
        "lit_extract_statements",
        "lit_get_paper",
        "lit_oeis",
        "lit_search_arxiv",
        "web_read",
        "web_research",
        "web_search",
    ] {
        assert!(
            listed.iter().any(|n| n == expected),
            "missing {expected}: {listed:?}"
        );
    }
    assert_eq!(listed.len(), 30);
    assert!(
        listed.iter().all(|n| !n.starts_with("agent_")),
        "{listed:?}"
    );
}

#[tokio::test]
async fn annotations_publish_read_only_exposure_and_kind() {
    let client = connect(Env::new().toolkit()).await;
    let tools = client.list_all_tools().await.unwrap();
    let find = |name: &str| tools.iter().find(|t| t.name == name).unwrap();
    for (name, read_only) in [
        ("calc_factor", true),
        ("lean_check", true),
        ("file_write", false),
        ("file_edit", false),
        ("ledger_record", false),
        ("ledger_link", false),
        ("ledger_bound", false),
    ] {
        let hint = find(name).annotations.as_ref().unwrap().read_only_hint;
        assert_eq!(hint, Some(read_only), "{name}");
    }
    let meta = |name: &str| find(name).meta.as_ref().unwrap().0["supermath"].clone();
    assert_eq!(meta("ledger_record")["exposure"], "direct");
    assert_eq!(meta("calc_factor")["exposure"], "deferred");
    assert_eq!(meta("web_search")["kind"], "agentic:gemini");
    assert_eq!(meta("lean_check")["kind"], "primitive");
}

#[tokio::test]
async fn with_no_lean_url_lean_check_is_backend_unavailable_and_the_catalogue_is_unchanged() {
    let bare = Env::new();
    bare.put(LEAN_PATH, CLEAN_LEAN);
    let client = connect(bare.toolkit()).await;
    let unconfigured = names(&client.list_all_tools().await.unwrap());

    let lean = LeanService::start().await;
    let configured = connect(Env::new().set("LEAN_URL", &lean.stub.base).toolkit()).await;
    assert_eq!(
        unconfigured,
        names(&configured.list_all_tools().await.unwrap())
    );

    let (result, envelope) = call(&client, "lean_check", json!({"path": LEAN_PATH})).await;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(envelope["error"]["code"], "BACKEND_UNAVAILABLE");
}

#[tokio::test]
async fn a_lean_claim_served_over_mcp_is_promoted_only_by_the_stubbed_check() {
    let lean = LeanService::start().await;
    let cortex = cortex().await;
    let env = Env::new()
        .set("LEAN_URL", &lean.stub.base)
        .set("CORTEX_API_URL", cortex.url())
        .set("CORTEX_API_KEY", "key");
    env.put(LEAN_PATH, CLEAN_LEAN);
    let client = connect(env.toolkit()).await;
    let (_, read) = call(&client, "file_read", json!({"path": LEAN_PATH})).await;
    let sha = read["data"]["sha256"].as_str().unwrap().to_owned();
    let source =
        json!({"lean": {"path": LEAN_PATH, "declaration": "two_add_two", "file_sha256": sha}});

    let (forged, forged_envelope) = call(
        &client,
        "ledger_record",
        json!({"kind": "result", "statement": "s", "source": source, "status": "formalised"}),
    )
    .await;
    assert_eq!(forged.is_error, Some(true));
    assert_eq!(forged_envelope["error"]["code"], "FORBIDDEN");
    assert_eq!(lean.stub.count("/check"), 0);

    let (promoted, envelope) = call(
        &client,
        "ledger_record",
        json!({"kind": "result", "statement": "s", "source": source}),
    )
    .await;
    assert_eq!(promoted.is_error, Some(false));
    assert_eq!(envelope["data"]["status"], "formalised");
    assert_eq!(lean.stub.count("/check"), 1);
}

#[tokio::test]
async fn every_result_carries_the_call_id_that_a_tool_source_quotes() {
    let cortex = cortex().await;
    let env = Env::new()
        .set("CORTEX_API_URL", cortex.url())
        .set("CORTEX_API_KEY", "key");
    let client = connect(env.toolkit()).await;
    let (_, ran) = call(&client, "calc_factor", json!({"n": 360})).await;
    let call_id = ran["call_id"].as_str().unwrap().to_owned();
    let (_, recorded) = call(
        &client,
        "ledger_record",
        json!({"kind": "result", "statement": "360 factors", "frame": "n = 360",
            "source": {"tool": {"tool": "calc_factor", "call_id": call_id}}}),
    )
    .await;
    assert_eq!(recorded["data"]["status"], "checked", "{recorded}");
}
