//! The host: the composed toolkit and the `agent_*` tools, as `OpenHuman`
//! tools.
//!
//! [`Host`] holds the [`Toolkit`] the agent links in process and the
//! `agent_*` tools built on the same runtime. [`Host::adapt`] wraps each in a
//! [`HostTool`]; a call to either goes through [`Host::dispatch`], so the one
//! call log the ledger reads records it.
//!
//! Primitive calls are dispatched by the toolkit and carry its `call-N` ids.
//! The `agent_*` tools are not in the toolkit's registry, so the host
//! dispatches them itself under `agent-N` ids, with an agentic budget and an
//! hour of wall time, because a child turn runs for minutes.
//!
//! The runner links the toolkit in process rather than connecting to the MCP
//! server: there is no second process to supervise, and the direct and
//! deferred split survives, where `OpenHuman` marks every MCP tool deferred.

mod types;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;
use supermath_mcp::{Dispatched, Toolkit};
use supermath_tool::{Budget, Tool, ToolRegistry};

use crate::error::{Error, Result};

pub use types::HostTool;

/// Wall time one `agent_*` call may take: an hour.
const AGENT_WALL_TIME_MS: u64 = 3_600_000;

/// The toolkit and the `agent_*` tools one agent can call.
#[derive(Debug)]
pub struct Host {
    toolkit: Arc<Toolkit>,
    agents: ToolRegistry,
    next_agent_call: AtomicU64,
}

impl Host {
    /// A host over `toolkit` and the `agent_*` tools built for it.
    ///
    /// # Errors
    ///
    /// [`Error::DuplicateTool`] when an agent tool shares a name with a
    /// toolkit tool or with another agent tool.
    pub fn new(toolkit: Arc<Toolkit>, agent_tools: &[Arc<dyn Tool>]) -> Result<Self> {
        let mut agents = ToolRegistry::new();
        for tool in agent_tools {
            let name = tool.name();
            if toolkit.registry().get(name).is_some() {
                return Err(Error::DuplicateTool(name.to_owned()));
            }
            agents
                .register(Arc::clone(tool))
                .map_err(|_| Error::DuplicateTool(name.to_owned()))?;
        }
        Ok(Self {
            toolkit,
            agents,
            next_agent_call: AtomicU64::new(1),
        })
    }

    /// The composed toolkit.
    #[must_use]
    pub fn toolkit(&self) -> &Toolkit {
        &self.toolkit
    }

    /// Every tool name the host serves: the toolkit's, then the `agent_*`
    /// tools', each in name order.
    #[must_use]
    pub fn tool_names(&self) -> Vec<&'static str> {
        self.toolkit
            .registry()
            .names()
            .chain(self.agents.names())
            .collect()
    }

    /// Every tool as an `OpenHuman` tool, in [`Host::tool_names`] order.
    #[must_use]
    pub fn adapt(self: &Arc<Self>) -> Vec<HostTool> {
        let registry = self.toolkit.registry();
        registry
            .iter()
            .chain(self.agents.iter())
            .map(|tool| HostTool::new(Arc::clone(self), Arc::clone(tool)))
            .collect()
    }

    /// [`Host::adapt`], boxed as the belt `OpenHuman` takes each turn.
    #[must_use]
    pub fn belt(self: &Arc<Self>) -> Vec<Box<dyn openhuman_embed::Tool>> {
        self.adapt()
            .into_iter()
            .map(|tool| Box::new(tool) as Box<dyn openhuman_embed::Tool>)
            .collect()
    }

    /// Calls `name` with `args`, records the call in the shared log, and
    /// returns the envelope with the call's id. An unknown name answers
    /// `NOT_FOUND`.
    pub async fn dispatch(&self, name: &str, args: Value) -> Dispatched {
        if self.agents.get(name).is_none() {
            return self.toolkit.dispatch(name, args).await;
        }
        let number = self.next_agent_call.fetch_add(1, Ordering::Relaxed);
        let call_id = format!("agent-{number}");
        let ctx = self.toolkit.context(call_id.clone()).with_budget(Budget {
            wall_time_ms: AGENT_WALL_TIME_MS,
            ..Budget::agentic(64, 1_000_000)
        });
        let envelope = self
            .agents
            .dispatch(self.toolkit.call_log().as_ref(), name, &ctx, args)
            .await;
        Dispatched { call_id, envelope }
    }
}

#[cfg(test)]
mod test;
