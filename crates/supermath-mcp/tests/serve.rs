//! `supermath-mcp serve`, as a client such as Claude Code runs it: the real
//! binary, a real MCP handshake over its standard input and output, the tool
//! list, and two calls.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use serde_json::{Value, json};
use tokio::process::Command;

fn arguments(value: &Value) -> serde_json::Map<String, Value> {
    value.as_object().cloned().unwrap()
}

fn envelope(result: &rmcp::model::CallToolResult) -> Value {
    serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
}

#[tokio::test]
async fn the_binary_handshakes_lists_tools_and_serves_calls() {
    let projects = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_supermath-mcp"));
    command
        .args(["serve", "My Project"])
        .env_clear()
        .env("SUPERMATH_PROJECTS", projects.path());
    support::keep_coverage_env(command.as_std_mut());
    let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();

    let info = client.peer_info().unwrap();
    assert_eq!(info.server_info.as_ref().unwrap().name, "supermath-mcp");

    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 30);
    assert!(tools.iter().any(|t| t.name == "file_write"));
    assert!(tools.iter().all(|t| !t.name.starts_with("agent_")));

    let factor = client
        .call_tool(
            CallToolRequestParams::new("calc_factor").with_arguments(arguments(&json!({"n": 360}))),
        )
        .await
        .unwrap();
    assert_eq!(factor.is_error, Some(false));
    let factored = envelope(&factor);
    assert_eq!(factored["outcome"], "ok");
    // `serve` keeps its call log on disk, so ids are unique across processes.
    let call_id = factored["call_id"].as_str().unwrap();
    assert!(call_id.starts_with("call-") && call_id.ends_with("-1"), "{call_id}");
    assert!(factored["data"].to_string().contains('5'), "{factored}");

    let written = client
        .call_tool(
            CallToolRequestParams::new("file_write").with_arguments(arguments(&json!({
                "path": "lean/Supermath/Hello.lean",
                "content": "import Mathlib\n\ntheorem t : True := trivial\n"
            }))),
        )
        .await
        .unwrap();
    assert_eq!(written.is_error, Some(false), "{}", envelope(&written));
    let on_disk = projects.path().join("my-project/lean/Supermath/Hello.lean");
    assert!(
        on_disk.is_file(),
        "the project is the slug of the argv name"
    );

    // A project named in an argument is not a thing: the schema forbids it.
    let escape = client
        .call_tool(
            CallToolRequestParams::new("file_write").with_arguments(arguments(&json!({
                "project": "other", "path": "lean/X.lean", "content": "x"
            }))),
        )
        .await
        .unwrap();
    assert_eq!(escape.is_error, Some(true));
    assert!(!projects.path().join("other").exists());

    client.cancel().await.unwrap();
}
