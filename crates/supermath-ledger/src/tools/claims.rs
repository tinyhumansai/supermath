//! `ledger_claims`: filter and page the ledger, or read one claim in full.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Coverage, Demand, Envelope, ErrorCode, Exposure, Kind as ToolKind, Permission,
    Tool, clamp_limit,
};

use super::{entry_json, failure, parse, trim};
use crate::error::Error;
use crate::ledger::Ledger;
use crate::types::{Alignment, Entry, Kind, Status};

/// The longest statement a list row carries.
const STATEMENT_LIMIT: usize = 300;

/// Lists and reads the project's ledger.
#[derive(Debug, Clone)]
pub struct LedgerClaims {
    ledger: Arc<Ledger>,
}

impl LedgerClaims {
    /// A `ledger_claims` tool over `ledger`.
    #[must_use]
    pub fn new(ledger: Arc<Ledger>) -> Self {
        Self { ledger }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    id: Option<String>,
    kind: Option<Kind>,
    min_status: Option<Status>,
    alignment: Option<Alignment>,
    query: Option<String>,
    limit: Option<u64>,
    cursor: Option<String>,
}

#[async_trait]
impl Tool for LedgerClaims {
    fn name(&self) -> &'static str {
        "ledger_claims"
    }

    fn description(&self) -> &'static str {
        "Given filters (kind, minimum status, alignment, or a text query), returns this \
         project's ledger entries in recall order, closest to the query first; given an \
         id, returns that claim in full with its effective status."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "string", "description": "One claim, in full."},
                "kind": {"type": "string",
                    "enum": ["fact", "result", "intake", "bound", "lead", "dead_end", "refutation"]},
                "min_status": {"type": "string",
                    "enum": ["asserted", "cited", "checked", "formalised"],
                    "description": "Keep claims whose effective status is at least this."},
                "alignment": {"type": "string",
                    "enum": ["not_reviewed", "accepted", "disputed"]},
                "query": {"type": "string", "description": "What the claims are about."},
                "limit": {"type": "integer", "minimum": 1, "maximum": 50},
                "cursor": {"type": "string"}
            },
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Read
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
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let args: Args = match parse(args) {
            Ok(args) => args,
            Err(error) => return failure(&error),
        };
        // Arguments are checked before the budget, so a malformed call is
        // `INVALID_INPUT` whatever budget it arrives with.
        let start = match start_of(&args) {
            Ok(start) => start,
            Err(error) => return failure(&error),
        };
        let limit = clamp_limit(args.limit);
        let demand = Demand {
            rows: limit,
            ..Demand::default()
        };
        if let Err(refusal) = ctx.budget.admit(&demand) {
            return refusal.into();
        }
        let result = match &args.id {
            Some(id) => self.one(ctx, id).await,
            None => self.list(ctx, &args, start, limit).await,
        };
        result.unwrap_or_else(|error| failure(&error))
    }
}

/// Where a list starts, after checking that `id` comes alone and that
/// `cursor` is one this tool issued.
fn start_of(args: &Args) -> Result<usize, Error> {
    if args.id.is_some()
        && (args.kind.is_some()
            || args.min_status.is_some()
            || args.alignment.is_some()
            || args.query.is_some()
            || args.cursor.is_some())
    {
        return Err(Error::InvalidArguments(
            "`id` cannot be combined with filters".into(),
        ));
    }
    match args.cursor.as_deref() {
        None => Ok(0),
        Some(cursor) => cursor
            .parse::<usize>()
            .map_err(|_| Error::InvalidArguments("`cursor` is not one this tool issued".into())),
    }
}

impl LedgerClaims {
    async fn one(&self, ctx: &CallContext, id: &str) -> Result<Envelope, Error> {
        let slug = ctx.project.slug();
        let entry = self
            .ledger
            .get(slug, id)
            .await?
            .ok_or_else(|| Error::ClaimNotFound(id.to_string()))?;
        let effective = self.ledger.effective_of(slug, &entry).await?;
        let mut envelope = Envelope::ok(json!({
            "claim": entry_json(&entry),
            "effective_status": effective.status,
            "limited_by": effective.chain,
        }));
        for note in effective.notes {
            envelope = envelope.with_unknown(note);
        }
        Ok(envelope)
    }

    async fn list(
        &self,
        ctx: &CallContext,
        args: &Args,
        start: usize,
        limit: usize,
    ) -> Result<Envelope, Error> {
        let slug = ctx.project.slug();
        let query = args.query.as_deref().map_or("", str::trim);
        let recalled = self.ledger.recall(slug, query).await?;
        let filtered: Vec<Entry> = recalled
            .entries
            .iter()
            .filter(|e| args.kind.is_none_or(|k| e.kind == k))
            .filter(|e| args.alignment.is_none_or(|a| e.alignment == a))
            .cloned()
            .collect();
        let effective = self.ledger.effective(slug, &filtered).await?;
        let rows: Vec<(Entry, Status)> = filtered
            .into_iter()
            .zip(effective)
            .filter(|(_, eff)| args.min_status.is_none_or(|min| eff.status >= min))
            .map(|(entry, eff)| (entry, eff.status))
            .collect();
        let page: Vec<Value> = rows
            .iter()
            .skip(start)
            .take(limit)
            .map(|(e, status)| {
                json!({
                    "id": e.id,
                    "kind": e.kind,
                    "status": status,
                    "alignment": e.alignment,
                    "statement": trim(&e.statement, STATEMENT_LIMIT),
                })
            })
            .collect();
        let end = start + page.len();
        let mut envelope = Envelope::ok(json!({"claims": page}))
            .with_coverage(Coverage::new(["project ledger"], ["library scope"]));
        if end < rows.len() {
            envelope = envelope.with_cursor(end.to_string());
        }
        if recalled.window_full {
            envelope = envelope.with_unknown(self.ledger.window_note());
        }
        Ok(envelope)
    }
}
