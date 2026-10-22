//! The ledger's tools: `recall_memory`, `ledger_record`, `ledger_claims`,
//! `ledger_link`, and `ledger_bound`.
//!
//! Each tool is a thin shell over [`Ledger`] or [`Memory`]: it parses and
//! checks its arguments, admits the call against the budget, calls the
//! ledger, and shapes the answer into an [`Envelope`]. The trust rules live
//! in the ledger, not here, so no tool can bypass them.

mod bound;
mod claims;
mod link;
mod recall;
mod record;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use supermath_tool::{Envelope, Tool};

use crate::error::{Error, Result};
use crate::ledger::Ledger;
use crate::memory::Memory;
use crate::types::Entry;

pub use bound::LedgerBound;
pub use claims::LedgerClaims;
pub use link::LedgerLink;
pub use recall::RecallMemory;
pub use record::LedgerRecord;

/// Every tool of this crate, over one ledger.
///
/// `recall_memory` reads `memory`; the four `ledger_*` tools use `ledger`.
#[must_use]
pub fn tools(memory: &Memory, ledger: &Arc<Ledger>) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(RecallMemory::new(memory.clone())),
        Arc::new(LedgerRecord::new(Arc::clone(ledger))),
        Arc::new(LedgerClaims::new(Arc::clone(ledger))),
        Arc::new(LedgerLink::new(Arc::clone(ledger))),
        Arc::new(LedgerBound::new(Arc::clone(ledger))),
    ]
}

/// Parses tool arguments into `T`.
fn parse<T: DeserializeOwned>(args: Value) -> Result<T> {
    serde_json::from_value(args).map_err(|error| Error::InvalidArguments(error.to_string()))
}

/// The envelope a failed call answers with. A Lean check that ran out of
/// time is `unknown`, not an error: it says nothing about the claim.
fn failure(error: &Error) -> Envelope {
    match error {
        Error::EvidenceTimeout => {
            Envelope::unknown("the lean check timed out, so nothing was recorded")
        }
        other => Envelope::error(other.code(), other.to_string()),
    }
}

/// The JSON schema of a `source`: exactly one of five shapes.
fn source_schema() -> Value {
    let string = json!({"type": "string"});
    json!({
        "description": "Where the claim came from: exactly one of arxiv, lean, tool, web, supplied.",
        "oneOf": [
            {"type": "object", "additionalProperties": false, "required": ["arxiv"],
             "properties": {"arxiv": {"type": "object", "additionalProperties": false,
                "description": "A paper and a locator in it: exactly one of label or section.",
                "required": ["id", "version"],
                "oneOf": [{"required": ["label"]}, {"required": ["section"]}],
                "properties": {"id": string, "version": string, "label": string, "section": string}}}},
            {"type": "object", "additionalProperties": false, "required": ["lean"],
             "properties": {"lean": {"type": "object", "additionalProperties": false,
                "required": ["path", "declaration", "file_sha256"],
                "properties": {"path": string, "declaration": string, "file_sha256": string}}}},
            {"type": "object", "additionalProperties": false, "required": ["tool"],
             "properties": {"tool": {"type": "object", "additionalProperties": false,
                "required": ["tool", "call_id"],
                "properties": {"tool": string, "call_id": string, "args_digest": string}}}},
            {"type": "object", "additionalProperties": false, "required": ["web"],
             "properties": {"web": {"type": "object", "additionalProperties": false,
                "required": ["url", "title", "class", "retrieved_at"],
                "properties": {"url": string, "title": string, "class": string, "retrieved_at": string}}}},
            {"type": "object", "additionalProperties": false, "required": ["supplied"],
             "properties": {"supplied": string}}
        ]
    })
}

/// The JSON of `entry` as a tool returns it in full.
fn entry_json(entry: &Entry) -> Value {
    serde_json::to_value(entry).unwrap_or(Value::Null)
}

/// `text` cut to at most `limit` characters, with an ellipsis if it was cut.
fn trim(text: &str, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod test;
