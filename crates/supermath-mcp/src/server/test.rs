//! The server over an in-memory pipe, with a real MCP client.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use rmcp::model::CallToolRequestParams;
use rmcp::{ServerHandler, ServiceExt};
use serde_json::{Value, json};

use super::{McpServer, serve_on};
use crate::{Config, Toolkit};

fn toolkit() -> Arc<Toolkit> {
    let config = Config::from_lookup(|_| None).unwrap();
    Arc::new(Toolkit::compose(&config, "demo").unwrap())
}

#[test]
fn the_server_names_itself_and_enables_tools() {
    let info = McpServer::new(toolkit()).get_info();
    assert_eq!(info.server_info.name, "supermath-mcp");
    assert!(info.capabilities.tools.is_some());
    assert!(info.instructions.is_some());
}

#[tokio::test]
async fn a_client_lists_and_calls_tools_over_a_pipe() {
    let (client_io, server_io) = tokio::io::duplex(1 << 16);
    let toolkit = toolkit();
    let server = tokio::spawn(serve_on(toolkit.clone(), server_io));
    let client = ().serve(client_io).await.unwrap();

    let tools = client.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), toolkit.registry().len());

    let mut arguments = serde_json::Map::new();
    arguments.insert("n".into(), json!(91));
    let result = client
        .call_tool(CallToolRequestParams::new("calc_factor").with_arguments(arguments))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let text = result.content[0].as_text().unwrap().text.clone();
    let envelope: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(envelope["outcome"], "ok");
    assert_eq!(envelope["call_id"], "call-1");

    let unknown = client
        .call_tool(CallToolRequestParams::new("nope"))
        .await
        .unwrap();
    assert_eq!(unknown.is_error, Some(true));

    client.cancel().await.unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_failed_handshake_is_a_transport_error() {
    let (client_io, server_io) = tokio::io::duplex(64);
    drop(client_io);
    let error = serve_on(toolkit(), server_io).await.unwrap_err();
    assert!(matches!(error, crate::Error::Transport(_)), "{error}");
}
