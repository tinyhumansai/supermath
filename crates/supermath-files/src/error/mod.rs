//! The crate-wide error type.
//!
//! These are *host-side* failures of the workspace: a bad path, a stale
//! digest, a disk error. A tool call never returns one; [`Error::code`] maps
//! each to the [`ErrorCode`] its envelope carries.

use supermath_tool::ErrorCode;

use crate::path::Language;

/// Convenience alias for results with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Everything fallible in this crate except a tool call.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A project slug was empty or held characters outside `[a-z0-9-]`.
    #[error("invalid project slug {0:?}")]
    InvalidSlug(String),
    /// A path broke the path rules.
    #[error("invalid path {path:?}: {reason}")]
    InvalidPath {
        /// The path as given.
        path: String,
        /// Which rule it broke.
        reason: &'static str,
    },
    /// A file imports something its language does not allow.
    #[error("{language} file imports {module:?}, which is not allowed: {reason}")]
    ForbiddenImport {
        /// The file's language.
        language: Language,
        /// The module as written.
        module: String,
        /// Why it is refused.
        reason: &'static str,
    },
    /// A path inside a generated directory, written by a tool that does not
    /// own it.
    #[error("{0:?} is in a generated directory that only its owning tool writes")]
    GeneratedPath(String),
    /// The file (or project) does not exist.
    #[error("{0:?} does not exist")]
    NotFound(String),
    /// The file's digest is not the one the caller expected.
    #[error("{path:?} changed: expected {expected}, found {actual}")]
    Conflict {
        /// The path.
        path: String,
        /// What the caller expected: a digest, or `no file`.
        expected: String,
        /// What is there: a digest, or `no file`.
        actual: String,
    },
    /// Content over the size cap.
    #[error("{bytes} bytes is over the {limit} byte limit")]
    TooLarge {
        /// The size asked for.
        bytes: u64,
        /// The cap.
        limit: u64,
    },
    /// The file is not UTF-8.
    #[error("{0:?} is not valid UTF-8")]
    NotUtf8(String),
    /// A disk operation failed.
    #[error("{op} failed: {source}")]
    Io {
        /// What was being done.
        op: &'static str,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// A line of the write log is not a valid record.
    #[error("write log line {line} is malformed: {source}")]
    LogCorrupt {
        /// One-based line number.
        line: usize,
        /// The decode error.
        #[source]
        source: serde_json::Error,
    },
}

impl Error {
    /// The [`ErrorCode`] a tool reports for this failure.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidSlug(_)
            | Self::InvalidPath { .. }
            | Self::ForbiddenImport { .. }
            | Self::GeneratedPath(_)
            | Self::NotUtf8(_) => ErrorCode::InvalidInput,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Conflict { .. } => ErrorCode::Conflict,
            Self::TooLarge { .. } => ErrorCode::BudgetExceeded,
            Self::Io { .. } | Self::LogCorrupt { .. } => ErrorCode::BackendUnavailable,
        }
    }

    pub(crate) fn io(op: &'static str, source: std::io::Error) -> Self {
        Self::Io { op, source }
    }
}

#[cfg(test)]
mod test;
