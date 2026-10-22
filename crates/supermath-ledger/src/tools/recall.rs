//! `recall_memory`: what earlier runs recorded about the current project.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Coverage, Demand, Envelope, ErrorCode, Exposure, Kind, Permission, Tool,
};

use super::{failure, parse, trim};
use crate::error::Error;
use crate::memory::{Memory, Project};

/// The most characters of rendered notes one call returns.
const NOTES_LIMIT: usize = 8_000;

/// Recalls what memory holds for the call's project. The scope comes from
/// the host's [`CallContext`], never from an argument.
#[derive(Debug, Clone)]
pub struct RecallMemory {
    memory: Memory,
}

impl RecallMemory {
    /// A `recall_memory` tool reading through `memory`.
    #[must_use]
    pub fn new(memory: Memory) -> Self {
        Self { memory }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    query: String,
}

#[async_trait]
impl Tool for RecallMemory {
    fn name(&self) -> &'static str {
        "recall_memory"
    }

    fn description(&self) -> &'static str {
        "Given a query, returns what earlier runs recorded about this project: notes, \
         extracted facts, and beliefs, found by similarity."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "What to look for."}
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Direct
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[ErrorCode::InvalidInput, ErrorCode::BackendUnavailable]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let args: Args = match parse(args) {
            Ok(args) => args,
            Err(error) => return failure(&error),
        };
        let query = args.query.trim();
        if query.is_empty() {
            return failure(&Error::InvalidArguments(
                "`query` must be a non-empty string".into(),
            ));
        }
        let demand = Demand {
            bytes: query.len() as u64,
            ..Demand::default()
        };
        if let Err(refusal) = ctx.budget.admit(&demand) {
            return refusal.into();
        }
        let project = match Project::new(ctx.project.slug()) {
            Ok(project) => project,
            Err(error) => return failure(&error),
        };
        let coverage = Coverage::new(["project memory"], ["library scope"]);
        match self.memory.recall(&project, query).await {
            Ok(Some(notes)) => {
                Envelope::ok(json!({"notes": trim(&notes, NOTES_LIMIT)})).with_coverage(coverage)
            }
            Ok(None) => Envelope::ok(json!({"notes": ""}))
                .with_coverage(coverage)
                .with_unknown(
                    "nothing matched; recall is by similarity, so this does not mean nothing \
                     was ever recorded",
                ),
            Err(error) => failure(&error),
        }
    }
}
