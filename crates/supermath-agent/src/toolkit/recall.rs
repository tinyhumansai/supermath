//! `recall_memory`: what earlier runs recorded about the current problem.
//!
//! The tool is built for one problem and holds that problem's scope. Its only
//! argument is the query; nothing the model sends can change which memory it
//! reads.

use openhuman_embed::{Tool, ToolPolicy, ToolResult};
use serde_json::{Value, json};

use crate::memory::Memory;
use crate::problem::Problem;

/// The name the model calls the tool by.
pub const RECALL_TOOL: &str = "recall_memory";

/// Recalls what memory holds for one problem.
#[derive(Debug, Clone)]
pub(crate) struct RecallTool {
    memory: Memory,
    problem: Problem,
}

impl RecallTool {
    pub(crate) fn new(memory: Memory, problem: Problem) -> Self {
        Self { memory, problem }
    }
}

#[async_trait::async_trait]
impl Tool for RecallTool {
    fn name(&self) -> &str {
        RECALL_TOOL
    }

    fn description(&self) -> &'static str {
        "Search what earlier runs recorded about the current problem. \
         Returns notes, extracted facts, and beliefs, or says nothing matched."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "What to look for." }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    fn policy(&self) -> ToolPolicy {
        ToolPolicy::read_only()
    }

    /// A failed recall is a tool error the model can read, not a failed
    /// turn: the run can go on without memory, but must be told it has none.
    async fn execute(&self, arguments: Value) -> anyhow::Result<ToolResult> {
        let Some(query) = arguments
            .get("query")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|query| !query.is_empty())
        else {
            return Ok(ToolResult::error("`query` must be a non-empty string"));
        };
        Ok(match self.memory.recall(&self.problem, query).await {
            Ok(Some(notes)) => ToolResult::success(notes),
            Ok(None) => ToolResult::success("Memory holds nothing matching that query."),
            Err(error) => ToolResult::error(format!("memory is unavailable: {error}")),
        })
    }
}
