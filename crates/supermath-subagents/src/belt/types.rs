//! Types the belt journals and reports.

use serde_json::Value;
use supermath_tool::{Envelope, Outcome};

/// One call a child made, as the registry answered it.
#[derive(Debug, Clone)]
pub struct CallNote {
    /// The primitive that was called.
    pub tool: String,
    /// The call id it was logged under in the [`CallLog`](supermath_tool::CallLog).
    pub call_id: String,
    /// The arguments the child sent.
    pub args: Value,
    /// What the registry answered.
    pub envelope: Envelope,
}

/// Receives a child's tool events, so an operator watching the parent sees
/// `agent_formalize -> lean_check` without the child's context reaching the
/// parent model.
pub trait Progress: Send + Sync {
    /// A child of `parent` started calling `tool`.
    fn tool_started(&self, parent: &str, tool: &str);
    /// A child of `parent` finished calling `tool` with `outcome`.
    fn tool_ended(&self, parent: &str, tool: &str, outcome: Outcome);
}
