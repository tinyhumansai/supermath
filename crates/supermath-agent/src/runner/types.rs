//! [`Solver`]: a registered agent and what it was built from.

use std::sync::Arc;

use openhuman_embed::Agent;

use crate::host::Host;

/// The top-level agent for one project.
#[derive(Debug)]
pub struct Solver {
    pub(super) agent: Agent,
    pub(super) host: Arc<Host>,
    pub(super) scope: Vec<String>,
    pub(super) unavailable: Vec<String>,
}

impl Solver {
    /// The registered `OpenHuman` agent.
    #[must_use]
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /// The toolkit and `agent_*` tools the agent calls.
    #[must_use]
    pub fn host(&self) -> &Arc<Host> {
        &self.host
    }

    /// The tool names the agent's scope lists, `tool_search` last.
    #[must_use]
    pub fn scope(&self) -> &[String] {
        &self.scope
    }

    /// The `agent_*` tools left out because the toolkit lacks primitives
    /// they need.
    #[must_use]
    pub fn unavailable(&self) -> &[String] {
        &self.unavailable
    }
}
