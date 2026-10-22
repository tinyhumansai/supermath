//! `ledger_bound`: offer a bound on a quantity; bounds only improve.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Number, Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind as ToolKind, Permission, Tool,
};

use super::{failure, parse, source_schema};
use crate::ledger::{BoundRequest, Ledger};
use crate::types::{Direction, Entry, Source, Status};

/// Records a bound if it improves on the best so far.
#[derive(Debug, Clone)]
pub struct LedgerBound {
    ledger: Arc<Ledger>,
}

impl LedgerBound {
    /// A `ledger_bound` tool over `ledger`.
    #[must_use]
    pub fn new(ledger: Arc<Ledger>) -> Self {
        Self { ledger }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    quantity: String,
    direction: Direction,
    value: Number,
    source: Source,
    frame: Option<String>,
}

#[async_trait]
impl Tool for LedgerBound {
    fn name(&self) -> &'static str {
        "ledger_bound"
    }

    fn description(&self) -> &'static str {
        "Given a named quantity, a direction, a value, and a source, records the bound if \
         it improves on the best known and returns the best bound so far either way."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "quantity": {"type": "string", "minLength": 1},
                "direction": {"type": "string", "enum": ["upper", "lower"]},
                "value": {"type": "number"},
                "source": source_schema(),
                "frame": {"type": "string",
                    "description": "For a tool source: the finite range the result covered."}
            },
            "required": ["quantity", "direction", "value", "source"],
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
            ErrorCode::Forbidden,
            ErrorCode::Conflict,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let args: Args = match parse(args) {
            Ok(args) => args,
            Err(error) => return failure(&error),
        };
        let request = BoundRequest {
            quantity: args.quantity,
            direction: args.direction,
            value: args.value,
            source: args.source,
            frame: args.frame,
        };
        match self.ledger.bound(ctx, request).await {
            Ok(outcome) => {
                let recorded = &outcome.recorded.entry;
                let mut envelope = Envelope::ok(json!({
                    "improved": outcome.improved,
                    "best": best_json(&outcome.best, outcome.best_effective.status),
                    "recorded": {
                        "id": recorded.id,
                        "kind": recorded.kind,
                        "status": recorded.status,
                        "created": outcome.recorded.created,
                    },
                }));
                let notes = outcome
                    .recorded
                    .notes
                    .into_iter()
                    .chain(outcome.best_effective.notes);
                for note in notes {
                    envelope = envelope.with_unknown(note);
                }
                if outcome.window_full {
                    envelope = envelope.with_unknown(format!(
                        "{}, so an older, better bound may have been missed",
                        self.ledger.window_note()
                    ));
                }
                envelope
            }
            Err(error) => failure(&error),
        }
    }
}

/// The best bound as the tool answers with it. `status` is the effective
/// status, as `ledger_claims` reports it.
fn best_json(entry: &Entry, status: Status) -> Value {
    json!({
        "id": entry.id,
        "statement": entry.statement,
        "status": status,
        "value": entry.bound.as_ref().map(|b| &b.value),
    })
}
