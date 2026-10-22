//! [`HostTool`]: one `supermath_tool::Tool`, as an `OpenHuman` host tool.

use std::sync::Arc;

use openhuman_embed::{PermissionLevel, ToolExposure, ToolPolicy, ToolResult};
use serde_json::Value;
use supermath_tool::{Exposure, Kind, Outcome, Permission};

use super::Host;

/// A registry tool the agent calls by its own name and schema.
///
/// Everything the model sees passes through from the tool; the call itself
/// goes through [`Host::dispatch`], so the shared call log records it.
pub struct HostTool {
    host: Arc<Host>,
    tool: Arc<dyn supermath_tool::Tool>,
}

impl HostTool {
    pub(super) fn new(host: Arc<Host>, tool: Arc<dyn supermath_tool::Tool>) -> Self {
        Self { host, tool }
    }

    /// The tool's kind: `OpenHuman` has no such field, so it stays here for
    /// hosts and tests that need to tell primitives from `agent_*` tools.
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.tool.kind()
    }
}

impl std::fmt::Debug for HostTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HostTool").field(&self.tool.name()).finish()
    }
}

#[async_trait::async_trait]
impl openhuman_embed::Tool for HostTool {
    fn name(&self) -> &str {
        self.tool.name()
    }

    fn description(&self) -> &str {
        self.tool.description()
    }

    fn parameters_schema(&self) -> Value {
        self.tool.schema()
    }

    fn policy(&self) -> ToolPolicy {
        match self.tool.permission() {
            Permission::Read => ToolPolicy::read_only(),
            Permission::Write => {
                let mut policy = ToolPolicy::classified();
                policy.side_effects.writes_files = true;
                policy
            }
        }
    }

    fn permission_level(&self) -> PermissionLevel {
        match self.tool.permission() {
            Permission::Read => PermissionLevel::ReadOnly,
            Permission::Write => PermissionLevel::Write,
        }
    }

    fn exposure(&self) -> ToolExposure {
        match self.tool.exposure() {
            Exposure::Direct => ToolExposure::Direct,
            Exposure::Deferred => ToolExposure::Deferred,
        }
    }

    /// Dispatches through the host and returns the envelope as JSON with the
    /// call's id added as `call_id`, the id a `tool` source quotes. An `error`
    /// envelope is flagged as a failed call; `ok` and `unknown` are answers.
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let done = self.host.dispatch(self.tool.name(), args).await;
        let mut body = serde_json::to_value(&done.envelope)?;
        if let Value::Object(object) = &mut body {
            object.insert("call_id".to_owned(), Value::String(done.call_id));
        }
        Ok(supermath_subagents::tool_result(&done.envelope, body))
    }
}
