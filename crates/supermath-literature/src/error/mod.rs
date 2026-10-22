//! The crate-wide [`Error`] and its mapping onto the tool envelope.

use supermath_tool::{Envelope, ErrorCode};

/// What can go wrong reading literature.
///
/// Every variant maps to exactly one stable [`ErrorCode`] through
/// [`Error::code`], and a tool turns an error into an envelope with
/// `Envelope::from`; a rate limit keeps its `retry_after_ms`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The caller's arguments or an identifier failed a check.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The named paper or resource does not exist at its source.
    #[error("not found: {0}")]
    NotFound(String),
    /// A source could not be reached or answered nonsense.
    #[error("{source_name} is unavailable: {reason}")]
    Unavailable {
        /// The source, such as `arxiv`.
        source_name: String,
        /// Why, in a few lowercase words.
        reason: String,
    },
    /// A source did not answer within the configured time.
    #[error("{source_name} did not answer in time")]
    Timeout {
        /// The source, such as `arxiv`.
        source_name: String,
    },
    /// A source answered 429 or 503 and asked to be left alone for a while.
    #[error("{source_name} asked to slow down")]
    RateLimited {
        /// The source, such as `arxiv`.
        source_name: String,
        /// How long the source asked us to wait, in milliseconds.
        retry_after_ms: u64,
    },
    /// A source answered with a status this crate does not interpret.
    #[error("{source_name} answered with status {status}")]
    Upstream {
        /// The source, such as `oeis`.
        source_name: String,
        /// The HTTP status.
        status: u16,
    },
    /// A response or e-print could not be decoded.
    #[error("could not decode {what}: {reason}")]
    Decode {
        /// What was being decoded, such as `the arxiv atom feed`.
        what: String,
        /// Why it failed.
        reason: String,
    },
}

/// The crate-wide result alias.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The stable [`ErrorCode`] this error becomes in an envelope.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidInput(_) => ErrorCode::InvalidInput,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Unavailable { .. } | Self::Upstream { .. } | Self::Decode { .. } => {
                ErrorCode::BackendUnavailable
            }
            Self::Timeout { .. } => ErrorCode::Timeout,
            Self::RateLimited { .. } => ErrorCode::RateLimited,
        }
    }
}

impl From<Error> for Envelope {
    fn from(error: Error) -> Self {
        let retry = match &error {
            Error::RateLimited { retry_after_ms, .. } => Some(*retry_after_ms),
            _ => None,
        };
        let envelope = Envelope::error(error.code(), error.to_string());
        match retry {
            Some(ms) => envelope.with_retry_after_ms(ms),
            None => envelope,
        }
    }
}

#[cfg(test)]
mod test;
