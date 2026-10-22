//! The crate-wide [`Error`] and its mapping onto the tool envelope.

use supermath_tool::{Envelope, ErrorCode};

/// What can go wrong checking Lean or searching Mathlib.
///
/// Every variant maps to exactly one stable [`ErrorCode`] through
/// [`Error::code`]; a tool turns an error into an envelope with
/// `Envelope::from`. A rate limit keeps its `retry_after_ms`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The caller's arguments failed a check.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The file, declaration, or position does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The file changed since the caller last saw it.
    #[error("conflict: {0}")]
    Conflict(String),
    /// The call asked for more than its budget allows.
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
    /// The Lean service, or a hosted index, cannot be reached or answered
    /// nonsense.
    #[error("{service} is unavailable: {reason}")]
    Unavailable {
        /// The service, such as `lean` or `loogle`.
        service: String,
        /// Why, in a few lowercase words.
        reason: String,
    },
    /// The service or the call ran out of time; the result is unknown.
    #[error("{0}")]
    Timeout(String),
    /// A hosted index asked to be left alone for a while.
    #[error("{service} asked to slow down")]
    RateLimited {
        /// The index, such as `loogle`.
        service: String,
        /// How long it asked us to wait, in milliseconds.
        retry_after_ms: u64,
    },
    /// The workspace refused a path or a file operation.
    #[error(transparent)]
    Files(#[from] supermath_files::Error),
}

/// The crate-wide result alias.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The [`ErrorCode`] a tool reports for this failure.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidInput(_) => ErrorCode::InvalidInput,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::BudgetExceeded(_) => ErrorCode::BudgetExceeded,
            Self::Unavailable { .. } => ErrorCode::BackendUnavailable,
            Self::Timeout(_) => ErrorCode::Timeout,
            Self::RateLimited { .. } => ErrorCode::RateLimited,
            Self::Files(e) => e.code(),
        }
    }

    /// An [`Error::Unavailable`] for `service`.
    pub(crate) fn unavailable(service: &str, reason: impl Into<String>) -> Self {
        Self::Unavailable {
            service: service.to_owned(),
            reason: reason.into(),
        }
    }
}

impl From<&Error> for Envelope {
    fn from(error: &Error) -> Self {
        let envelope = Self::error(error.code(), error.to_string());
        match error {
            Error::RateLimited { retry_after_ms, .. } => {
                envelope.with_retry_after_ms(*retry_after_ms)
            }
            _ => envelope,
        }
    }
}

impl From<Error> for Envelope {
    fn from(error: Error) -> Self {
        Self::from(&error)
    }
}

#[cfg(test)]
mod test;
