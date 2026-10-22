//! What the eight tools share: argument parsing, admission, status mapping,
//! and the cancellation guard.

use std::future::Future;

use serde::de::DeserializeOwned;
use supermath_files::{Language, ProjectPath};
use supermath_tool::{CallContext, Coverage, Demand, Envelope, ErrorCode};

use crate::client::Status;
use crate::error::{Error, Result};

/// Decodes a tool's arguments, refusing unknown or mistyped fields.
pub(super) fn parse_args<T: DeserializeOwned>(args: serde_json::Value) -> Result<T> {
    serde_json::from_value(args).map_err(|e| Error::InvalidInput(format!("bad arguments: {e}")))
}

/// Parses a path argument and requires it to name a Lean file.
pub(super) fn parse_lean_path(raw: &str) -> Result<ProjectPath> {
    let path = ProjectPath::parse(raw)?;
    if path.language() == Language::Lean {
        Ok(path)
    } else {
        Err(Error::InvalidInput(format!("{path} is not a Lean file")))
    }
}

/// Admits a call that will send `bytes` bytes of source to the service.
pub(super) fn admit_bytes(ctx: &CallContext, bytes: usize) -> Result<()> {
    ctx.budget
        .admit(&Demand {
            bytes: bytes as u64,
            ..Demand::default()
        })
        .map_err(|refusal| Error::BudgetExceeded(refusal.to_string()))
}

/// Runs a tool body, turning an error into its envelope and a cancellation
/// into `TIMEOUT`.
pub(super) async fn guard(
    ctx: &CallContext,
    body: impl Future<Output = Result<Envelope>>,
) -> Envelope {
    tokio::select! {
        result = body => result.unwrap_or_else(Envelope::from),
        () = ctx.cancel.cancelled() => {
            Envelope::error(ErrorCode::Timeout, "the call was cancelled before it finished")
        }
    }
}

/// Maps a service status that is not `ok` to the error a tool reports.
/// `what` names the operation, such as `the goal query`.
pub(super) fn expect_ok(status: Status, what: &str) -> Result<()> {
    match status {
        Status::Ok => Ok(()),
        Status::Timeout => Err(Error::Timeout(format!("{what} ran out of time"))),
        Status::Memory => Err(Error::Timeout(format!("{what} ran out of memory"))),
        Status::OutputLimit => Err(Error::Timeout(format!("{what} passed the output size cap"))),
        Status::NoSorry => Err(Error::InvalidInput(
            "there is no sorry at that position".into(),
        )),
        Status::BadPosition => Err(Error::InvalidInput(
            "the position is outside the file".into(),
        )),
        Status::BadPredicate => Err(Error::InvalidInput(
            "the declaration is not a decidable predicate on integers".into(),
        )),
        Status::NotFound => Err(Error::NotFound(
            "the declaration is not in the environment".into(),
        )),
        Status::Other => Err(Error::unavailable(
            "lean",
            "the service sent an unknown status",
        )),
    }
}

/// Trims `text` to at most `max` characters, marking the cut with `…`.
pub(super) fn trim_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((cut, _)) => format!("{}…", text[..cut].trim_end()),
        None => text.to_owned(),
    }
}

/// A declaration name that is safe to send to the service and to put in a
/// generated file: letters, digits, `_`, `'`, `.`, `!`, `?`.
pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= 200
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '\'' | '.' | '!' | '?'))
}

/// A `cursor` argument: a decimal offset.
pub(super) fn parse_cursor(cursor: Option<&str>) -> Result<usize> {
    cursor.map_or(Ok(0), |c| {
        c.parse()
            .map_err(|_| Error::InvalidInput(format!("cursor {c:?} is not one this tool returned")))
    })
}

/// A [`Coverage`] of one thing searched and the things it left out.
pub(super) fn coverage(searched: impl Into<String>, not_covered: &[&str]) -> Coverage {
    Coverage::new(
        [searched.into()],
        not_covered.iter().map(|s| (*s).to_owned()),
    )
}
