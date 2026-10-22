//! The crate-wide error type.
//!
//! These are *host* errors: building a project, filling a registry, failing a
//! contract check. A tool call never returns one; a failed call is an `error`
//! [`Envelope`](crate::Envelope) with an [`ErrorCode`](crate::ErrorCode).

use crate::contract::Violation;

/// Convenience alias for results with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Everything fallible in this crate except a tool call.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// A project slug was empty or held characters outside `[a-z0-9-]`.
    #[error("invalid project slug {0:?}")]
    InvalidProjectSlug(String),
    /// A project's scope root was empty.
    #[error("project scope root is empty")]
    EmptyProjectScope,
    /// Two tools were registered under one name.
    #[error("tool {0:?} is already registered")]
    DuplicateTool(String),
    /// A registry lookup named a tool that is not registered.
    #[error("tool {0:?} is not registered")]
    UnknownTool(String),
    /// A tool broke the tool contract; carries every violation found.
    #[error("tool {tool:?} breaks the tool contract: {}", join(.violations))]
    Contract {
        /// The tool's name.
        tool: String,
        /// Every violation the checker found.
        violations: Vec<Violation>,
    },
}

fn join(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod test;
