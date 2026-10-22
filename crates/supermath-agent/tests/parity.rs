//! One tool, two hosts: the same arguments through `supermath-mcp`'s server
//! (the code `supermath-mcp serve` runs, over an in-memory pipe instead of
//! stdio) and through the runner's host tool give the same envelope. Only
//! the error flag differs, by design: the runner flags service failures, not
//! refusals.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use openhuman_embed::Tool as _;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};
use supermath_agent::Host;
use supermath_mcp::{Config, Toolkit, serve_on};

/// A toolkit over an empty projects directory, with no backend configured.
fn toolkit(projects: &std::path::Path) -> Toolkit {
    let projects = projects.display().to_string();
    let config =
        Config::from_lookup(|name| (name == "SUPERMATH_PROJECTS").then(|| projects.clone()))
            .unwrap();
    Toolkit::compose(&config, "Parity").unwrap()
}

/// The envelope and the error flag `supermath-mcp`'s server answers.
async fn over_mcp(projects: &std::path::Path, name: &str, args: &Value) -> (Value, bool) {
    let (client_io, server_io) = tokio::io::duplex(1 << 20);
    tokio::spawn(serve_on(Arc::new(toolkit(projects)), server_io));
    let client = ().serve(client_io).await.unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().cloned().unwrap()),
        )
        .await
        .unwrap();
    client.cancel().await.unwrap();
    let text = &result.content[0].as_text().unwrap().text;
    (
        serde_json::from_str(text).unwrap(),
        result.is_error == Some(true),
    )
}

/// The envelope and the error flag the runner's host tool answers.
async fn over_host(projects: &std::path::Path, name: &str, args: &Value) -> (Value, bool) {
    let host = Arc::new(Host::new(Arc::new(toolkit(projects)), &[]).unwrap());
    let tool = host
        .adapt()
        .into_iter()
        .find(|tool| tool.name() == name)
        .unwrap();
    let result = tool.execute(args.clone()).await.unwrap();
    let first = serde_json::to_value(&result).unwrap()["content"][0].clone();
    let body = match first.get("data") {
        Some(data) => data.clone(),
        None => serde_json::from_str(first["text"].as_str().unwrap()).unwrap(),
    };
    (body, result.is_error)
}

#[tokio::test]
async fn mcp_serve_and_the_host_tool_answer_the_same_envelope() {
    let projects = tempfile::tempdir().unwrap();
    for (name, args, failed) in [
        ("calc_factor", json!({"n": 360}), false),
        ("calc_factor", json!({"n": "ten"}), true),
        ("calc_gcd_lcm", json!({"a": 12, "b": 18}), false),
    ] {
        let mcp = over_mcp(projects.path(), name, &args).await;
        let host = over_host(projects.path(), name, &args).await;
        assert_eq!(mcp.0, host.0, "{name} {args}");
        assert_eq!(mcp.1, failed, "{name} {args}: {}", mcp.0);
        // The flags differ on purpose. To an MCP client `isError` is a hint;
        // to OpenHuman a failed call feeds the classifier that halts a turn,
        // and it read a refusal such as `FORBIDDEN` as rejected credentials.
        // So the runner sends a refusal as a result, `outcome: "error"`
        // intact, and flags only a service failure
        // (`supermath_subagents::tool_result`).
        assert!(!host.1, "{name} {args}: a refusal is not a host failure");
        assert_eq!(mcp.0["call_id"], "call-1");
    }
}
