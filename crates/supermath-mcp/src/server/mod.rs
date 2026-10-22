//! The MCP server: [`McpServer`] adapts a [`Toolkit`] to `rmcp`, and
//! [`serve`] speaks it over stdio.
//!
//! - `tools/list` lists every tool in the registry with its name,
//!   description, and input schema. MCP has no deferral, so nothing is held
//!   back. `readOnlyHint` is `true` for `Read` tools and `false` for the
//!   rest. MCP's annotations have no field for a tool's exposure or kind, so
//!   they travel in `_meta.supermath`: `{"exposure": "direct", "kind":
//!   "primitive"}`.
//! - `tools/call` runs the tool through [`Toolkit::dispatch`] and returns the
//!   envelope as one JSON text block, with the call's id added as `call_id`
//!   (a `ledger_record` `tool` source quotes it). `isError` is set when the
//!   envelope is an `error`.
//! - The project is fixed when the toolkit is composed, from the command
//!   line. No argument can name another one.
//!
//! Standard output belongs to the protocol: nothing else may print to it.

mod descriptor;

use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::transport::IntoTransport;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::toolkit::Toolkit;

/// An MCP server over a [`Toolkit`].
#[derive(Debug, Clone)]
pub struct McpServer {
    toolkit: Arc<Toolkit>,
}

impl McpServer {
    /// A server for `toolkit`.
    #[must_use]
    pub fn new(toolkit: Arc<Toolkit>) -> Self {
        Self { toolkit }
    }

    /// Every tool as MCP lists it.
    fn tools(&self) -> Vec<rmcp::model::Tool> {
        self.toolkit
            .registry()
            .iter()
            .map(|tool| descriptor::describe(tool.as_ref()))
            .collect()
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "supermath-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Math research tools for one project: project files, Lean and Python \
                 checks, arXiv and web search, a claim ledger, and exact arithmetic. \
                 Every result is an envelope with an outcome of ok, unknown, or error; \
                 unknown says nothing about the mathematics.",
            )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListToolsResult, ErrorData>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(self.tools())))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        let args = Value::Object(request.arguments.unwrap_or_default());
        let done = self.toolkit.dispatch(&request.name, args).await;
        Ok(descriptor::result(&done.call_id, &done.envelope).into())
    }
}

/// Serves `toolkit` over `transport` until the client disconnects.
///
/// # Errors
///
/// [`Error::Transport`] if the handshake fails or the connection breaks.
pub async fn serve_on<T, E, A>(toolkit: Arc<Toolkit>, transport: T) -> Result<()>
where
    T: IntoTransport<RoleServer, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let running = McpServer::new(toolkit)
        .serve(transport)
        .await
        .map_err(|error| Error::Transport(error.to_string()))?;
    running
        .waiting()
        .await
        .map_err(|error| Error::Transport(error.to_string()))?;
    Ok(())
}

/// Serves `toolkit` over this process's standard input and output until the
/// client closes it.
///
/// # Errors
///
/// As [`serve_on`].
pub async fn serve(toolkit: Arc<Toolkit>) -> Result<()> {
    serve_on(toolkit, rmcp::transport::stdio()).await
}

#[cfg(test)]
mod test;
