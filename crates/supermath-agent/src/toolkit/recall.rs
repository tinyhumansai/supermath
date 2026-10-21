//! The `recall_memory` host tool.
//!
//! The tool is built for one problem and holds that problem's scope. Its only
//! argument is the query; nothing the model sends can change which memory it
//! reads.

use openhuman_embed::{Tool, ToolPolicy, ToolResult};
use serde_json::{Value, json};

use crate::memory::Memory;
use crate::problem::Problem;

/// The name the model calls the tool by.
pub(crate) const RECALL_TOOL: &str = "recall_memory";

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

    /// Runs one recall and phrases the outcome for the model.
    ///
    /// A failed recall is a tool error the model can read, not a failed
    /// turn: the run can go on without memory, but must be told it has none.
    pub(crate) async fn answer(&self, arguments: &Value) -> ToolResult {
        let Some(query) = arguments
            .get("query")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|query| !query.is_empty())
        else {
            return ToolResult::error("`query` must be a non-empty string");
        };
        match self.memory.recall(&self.problem, query).await {
            Ok(Some(notes)) => ToolResult::success(notes),
            Ok(None) => ToolResult::success("Memory holds nothing matching that query."),
            Err(error) => ToolResult::error(format!("memory is unavailable: {error}")),
        }
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

    async fn execute(&self, arguments: Value) -> anyhow::Result<ToolResult> {
        Ok(self.answer(&arguments).await)
    }
}
