//! The crate-wide [`Error`] and its mapping onto the tool envelope.

use supermath_tool::{Envelope, ErrorCode};

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Why a Python tool could not produce an answer.
///
/// A tool call never returns one of these: [`Error::code`] and the
/// `From<Error> for Envelope` impl turn each into the envelope the agent sees.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The arguments are wrong, or the service refused the request as such.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The file, or something the service looked for, does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The call is over a time, size, or compute cap.
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
    /// The service is not configured, not reachable, or did not answer in time.
    #[error("the python service is unavailable: {0}")]
    Unavailable(String),
    /// The service answered with a status this crate does not expect.
    #[error("the python service answered with status {status}: {message}")]
    Upstream {
        /// The HTTP status.
        status: u16,
        /// The service's message, or the status text.
        message: String,
    },
    /// The service's answer did not have the documented shape.
    #[error("could not decode the python service's answer: {0}")]
    Decode(String),
    /// A workspace read or write failed.
    #[error(transparent)]
    Files(#[from] supermath_files::Error),
}

impl Error {
    /// The envelope code this error is reported under.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidInput(_) => ErrorCode::InvalidInput,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::BudgetExceeded(_) => ErrorCode::BudgetExceeded,
            Self::Unavailable(_) | Self::Upstream { .. } | Self::Decode(_) => {
                ErrorCode::BackendUnavailable
            }
            Self::Files(error) => error.code(),
        }
    }
}

impl From<Error> for Envelope {
    fn from(error: Error) -> Self {
        Envelope::error(error.code(), error.to_string())
    }
}

#[cfg(test)]
mod test;
