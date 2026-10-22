//! [`HostTool`]: one registry tool, as an `OpenHuman` host tool.

use std::sync::Arc;

use openhuman_embed::{PermissionLevel, ToolExposure, ToolPolicy, ToolResult};
use serde_json::Value;
use supermath_tool::Permission;

use super::Belt;

/// A [`supermath_tool::Tool`] the child calls by its own name and schema.
pub struct HostTool {
    belt: Arc<Belt>,
    tool: Arc<dyn supermath_tool::Tool>,
}

impl HostTool {
    /// Adapts `tool`; calls go through `belt` so they are logged and journaled.
    #[must_use]
    pub fn new(belt: Arc<Belt>, tool: Arc<dyn supermath_tool::Tool>) -> Self {
        Self { belt, tool }
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
        ToolExposure::Direct
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let envelope = self.belt.call(self.tool.name(), args).await;
        let body = serde_json::to_value(&envelope)?;
        Ok(super::tool_result(&envelope, body))
    }
}
