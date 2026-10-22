//! `ledger_link`: record that one claim uses another.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind as ToolKind, Permission, Tool,
};

use super::{failure, parse};
use crate::ledger::Ledger;

/// Adds a `uses` edge between two claims.
#[derive(Debug, Clone)]
pub struct LedgerLink {
    ledger: Arc<Ledger>,
}

impl LedgerLink {
    /// A `ledger_link` tool over `ledger`.
    #[must_use]
    pub fn new(ledger: Arc<Ledger>) -> Self {
        Self { ledger }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    from: String,
    to: String,
}

#[async_trait]
impl Tool for LedgerLink {
    fn name(&self) -> &'static str {
        "ledger_link"
    }

    fn description(&self) -> &'static str {
        "Given two claim ids, records that the first uses the second, so the first is \
         no stronger than the second. Refuses a cycle."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "from": {"type": "string", "description": "The claim that depends on another."},
                "to": {"type": "string", "description": "The claim it uses."}
            },
            "required": ["from", "to"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Write
    }

    fn kind(&self) -> ToolKind {
        ToolKind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::BackendUnavailable,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let args: Args = match parse(args) {
            Ok(args) => args,
            Err(error) => return failure(&error),
        };
        match self.ledger.link(ctx, &args.from, &args.to).await {
            Ok(recorded) => Envelope::ok(json!({
                "from": recorded.entry.id,
                "to": args.to,
                "uses": recorded.entry.uses,
                "created": recorded.created,
            })),
            Err(error) => failure(&error),
        }
    }
}
