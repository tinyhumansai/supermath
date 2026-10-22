//! `ledger_record`: record a claim or fact; the status is computed.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Demand, Envelope, ErrorCode, EvidenceOrigin, Exposure, Kind as ToolKind,
    Permission, Tool,
};

use super::{failure, parse, source_schema};
use crate::error::Error;
use crate::ledger::{Claim, Ledger, Recorded};
use crate::types::{Kind, Source, Status};

/// Records an entry in the project's ledger and returns its id and status.
#[derive(Debug, Clone)]
pub struct LedgerRecord {
    ledger: Arc<Ledger>,
}

impl LedgerRecord {
    /// A `ledger_record` tool over `ledger`.
    #[must_use]
    pub fn new(ledger: Arc<Ledger>) -> Self {
        Self { ledger }
    }
}

/// `status` and `evidence_origin` are accepted so that a caller who sends
/// them gets `FORBIDDEN` rather than a silent drop.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    kind: Kind,
    statement: String,
    source: Source,
    #[serde(default)]
    uses: Vec<String>,
    rationale: Option<String>,
    reason: Option<String>,
    refutes: Option<String>,
    frame: Option<String>,
    status: Option<Status>,
    evidence_origin: Option<EvidenceOrigin>,
}

#[async_trait]
impl Tool for LedgerRecord {
    fn name(&self) -> &'static str {
        "ledger_record"
    }

    fn description(&self) -> &'static str {
        "Given a kind, a statement, and a source, records a claim or fact in this \
         project's ledger and returns its id and the status the evidence earned."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "kind": {"type": "string",
                    "enum": ["fact", "result", "intake", "bound", "lead", "dead_end", "refutation"]},
                "statement": {"type": "string", "minLength": 1},
                "source": source_schema(),
                "uses": {"type": "array", "items": {"type": "string"},
                    "description": "Ids of the claims this one depends on."},
                "rationale": {"type": "string", "description": "For a lead: why it is worth trying."},
                "reason": {"type": "string", "description": "For a dead_end: why it failed."},
                "refutes": {"type": "string", "description": "For a refutation: the claim's id."},
                "frame": {"type": "string",
                    "description": "For a tool source: the finite range the result covered."},
                "status": {"type": "string",
                    "enum": ["asserted", "cited", "checked", "formalised"],
                    "description": "Never taken from the caller: anything above asserted is FORBIDDEN."},
                "evidence_origin": {"type": "string", "enum": ["collected", "supplied"],
                    "description": "Never taken from the caller: collected is FORBIDDEN."}
            },
            "required": ["kind", "statement", "source"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Direct
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
            ErrorCode::BudgetExceeded,
            ErrorCode::Forbidden,
            ErrorCode::Conflict,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let args: Args = match parse(args) {
            Ok(args) => args,
            Err(error) => return failure(&error),
        };
        // A caller never sets a status or an origin. Saying `asserted` or
        // `supplied` only lowers what the evidence could earn, so it is
        // allowed; anything higher is refused outright.
        if let Some(status) = args.status
            && status != Status::Asserted
        {
            return failure(&Error::StatusFromCaller(status.as_str().to_string()));
        }
        if args.evidence_origin == Some(EvidenceOrigin::Collected) {
            return failure(&Error::OriginFromCaller);
        }
        let demand = Demand {
            bytes: args.statement.len() as u64,
            ..Demand::default()
        };
        if let Err(refusal) = ctx.budget.admit(&demand) {
            return refusal.into();
        }
        let mut ctx = ctx.clone();
        if args.evidence_origin == Some(EvidenceOrigin::Supplied) {
            ctx.evidence_origin = EvidenceOrigin::Supplied;
        }
        if args.status == Some(Status::Asserted) {
            // An explicit `asserted` pins the claim there: no evidence runs.
            ctx.evidence_origin = EvidenceOrigin::Supplied;
        }
        let mut claim = Claim::new(args.kind, args.statement, args.source);
        claim.uses = args.uses;
        claim.rationale = args.rationale;
        claim.reason = args.reason;
        claim.refutes = args.refutes;
        claim.frame = args.frame;
        match self.ledger.record(&ctx, claim).await {
            Ok(recorded) => answer(&recorded),
            Err(error) => failure(&error),
        }
    }
}

/// The envelope for a recorded entry.
pub(super) fn answer(recorded: &Recorded) -> Envelope {
    let entry = &recorded.entry;
    let mut data = json!({
        "id": entry.id,
        "kind": entry.kind,
        "status": entry.status,
        "evidence_origin": entry.evidence_origin,
        "alignment": entry.alignment,
        "revision": entry.revision,
        "created": recorded.created,
    });
    if let Some(lean) = &entry.lean {
        data["lean"] = json!({"verdict": lean.verdict, "file_sha256": lean.file_sha256});
    }
    recorded
        .notes
        .iter()
        .fold(Envelope::ok(data), |envelope, note| {
            envelope.with_unknown(note.clone())
        })
}
