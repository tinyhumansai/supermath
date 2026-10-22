//! How an envelope reaches `OpenHuman`: as a result the model reads, or as a
//! failure the harness acts on.
//!
//! `OpenHuman` treats a failed call as a signal about the run. It classifies
//! the failure (from an explicit `status_code`, else by keywords in the
//! text) and halts the turn on a credential or uncertain-effect class with
//! no retry. A toolkit refusal is not such a signal: `FORBIDDEN` (a write
//! outside a duel side's area), `CONFLICT` (a stale `if_sha256`), or
//! `TIMEOUT` (a Lean check that ran long) is the tool's answer, and the
//! model is expected to read it and change course. Sent as a failure, its
//! text read as "credentials rejected" and ended the turn.
//!
//! So refusals go back as results, with the envelope's `outcome: "error"`
//! intact, and only a service failure (`BACKEND_UNAVAILABLE`,
//! `RATE_LIMITED`) is a failure, with the HTTP status the harness reads as
//! transient.

use openhuman_embed::ToolResult;
use serde_json::Value;
use supermath_tool::{Envelope, ErrorCode, Outcome};

/// The tool result for `envelope`, whose wire form is `body`.
#[must_use]
pub fn tool_result(envelope: &Envelope, mut body: Value) -> ToolResult {
    let status = match (envelope.outcome, envelope.code()) {
        (Outcome::Error, Some(ErrorCode::BackendUnavailable)) => 503,
        (Outcome::Error, Some(ErrorCode::RateLimited)) => 429,
        _ => return ToolResult::json(body),
    };
    if let Value::Object(object) = &mut body {
        object.insert("status_code".to_owned(), Value::from(status));
    }
    ToolResult::error(body.to_string())
}
