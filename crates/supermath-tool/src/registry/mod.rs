//! [`ToolRegistry`]: tools by name.
//!
//! The composition fills one with every domain's tools, and the MCP server,
//! the `call` command and the loop scaffold read it.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use crate::error::{Error, Result};
use crate::{CallContext, CallLog, CallRecord, Envelope, ErrorCode, Tool};

/// Tools by name, in name order.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<&'static str, Arc<dyn Tool>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.tools.keys()).finish()
    }
}

impl ToolRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a tool.
    ///
    /// # Errors
    ///
    /// [`Error::DuplicateTool`] if a tool with this name is registered.
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result<()> {
        let name = tool.name();
        if self.tools.contains_key(name) {
            return Err(Error::DuplicateTool(name.to_owned()));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Adds every tool of `other`.
    ///
    /// # Errors
    ///
    /// [`Error::DuplicateTool`] on the first name already registered; tools
    /// added before it stay.
    pub fn extend(&mut self, other: &ToolRegistry) -> Result<()> {
        other
            .iter()
            .try_for_each(|tool| self.register(Arc::clone(tool)))
    }

    /// The tool with this name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Every tool, in name order.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Tool>> {
        self.tools.values()
    }

    /// Every name, in order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.tools.keys().copied()
    }

    /// How many tools are registered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether no tool is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// A registry holding only the named tools: the scope of a child agent.
    ///
    /// # Errors
    ///
    /// [`Error::UnknownTool`] for the first name that is not registered.
    pub fn subset(&self, names: &[&str]) -> Result<ToolRegistry> {
        let mut subset = ToolRegistry::new();
        for name in names {
            let tool = self
                .get(name)
                .ok_or_else(|| Error::UnknownTool((*name).to_owned()))?;
            if subset.get(name).is_none() {
                subset.register(Arc::clone(tool))?;
            }
        }
        Ok(subset)
    }

    /// Calls the named tool and records the call in `log`. An unregistered
    /// name answers `NOT_FOUND`, and the call is logged too.
    pub async fn dispatch(
        &self,
        log: &dyn CallLog,
        name: &str,
        ctx: &CallContext,
        args: serde_json::Value,
    ) -> Envelope {
        let started = Instant::now();
        let envelope = match self.get(name) {
            Some(tool) => tool.call(ctx, args).await,
            None => Envelope::error(ErrorCode::NotFound, format!("no tool named {name}")),
        };
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        log.record(CallRecord::new(
            ctx.call_id.clone(),
            name,
            ctx.project.slug(),
            &envelope,
            elapsed_ms,
        ));
        envelope
    }
}

#[cfg(test)]
mod test;
