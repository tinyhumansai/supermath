//! What the four tools share: argument parsing, error envelopes, admission.

use serde::de::DeserializeOwned;
use supermath_tool::{CallContext, Demand, Envelope, ErrorCode};

use crate::error::Error;

/// A boxed error envelope, so `Result<_, Failure>` stays small.
pub(super) type Failure = Box<Envelope>;
use crate::path::{Language, ProjectPath};

/// Decodes a tool's arguments, refusing unknown or mistyped fields.
pub(super) fn parse_args<T: DeserializeOwned>(args: serde_json::Value) -> Result<T, Failure> {
    serde_json::from_value(args).map_err(|e| bad(format!("bad arguments: {e}")))
}

/// Parses a path argument.
pub(super) fn parse_path(raw: &str) -> Result<ProjectPath, Failure> {
    ProjectPath::parse(raw).map_err(|e| failure(&e))
}

/// The error envelope for a workspace failure.
pub(super) fn failure(error: &Error) -> Failure {
    Box::new(Envelope::error(error.code(), error.to_string()))
}

/// An `INVALID_INPUT` envelope.
pub(super) fn bad(message: impl Into<String>) -> Failure {
    Box::new(Envelope::error(ErrorCode::InvalidInput, message))
}

/// Admits a call that will read or write `bytes` bytes against its budget.
pub(super) fn admit_bytes(ctx: &CallContext, bytes: usize) -> Result<(), Failure> {
    ctx.budget
        .admit(&Demand {
            bytes: bytes as u64,
            ..Demand::default()
        })
        .map_err(|refusal| Box::new(Envelope::from(refusal)))
}

/// The follow-up call that checks or runs the file just written.
pub(super) fn follow_up(path: &ProjectPath) -> String {
    let tool = match path.language() {
        Language::Lean => "lean_check",
        Language::Python => "py_run",
    };
    format!("{tool} {{\"path\":\"{path}\"}}")
}
