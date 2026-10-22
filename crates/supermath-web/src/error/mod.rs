//! The crate-wide error type.
//!
//! Every fallible step of a web call returns [`Error`]. A tool turns it into
//! an `error` envelope with [`Error::code`]; the public building blocks
//! ([`GeminiClient`](crate::GeminiClient), [`resolve_redirect`](crate::resolve_redirect))
//! return it directly.

use supermath_tool::{Envelope, ErrorCode};

/// Convenience alias for results with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong in a web call.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// An environment variable held a value that cannot be used.
    #[error("invalid value for {variable}: {reason}")]
    InvalidConfig {
        /// The variable's name.
        variable: &'static str,
        /// Why the value was refused.
        reason: String,
    },
    /// The HTTP client could not be built.
    #[error("cannot build the http client: {0}")]
    Client(String),
    /// `GEMINI_API_KEY` is not set, and no other engine stands in.
    #[error("GEMINI_API_KEY is not set")]
    MissingApiKey,
    /// The upstream asked us to slow down.
    #[error("rate limited; retry after {retry_after_ms} ms")]
    RateLimited {
        /// When to retry, in milliseconds.
        retry_after_ms: u64,
    },
    /// The upstream is down or refused the request.
    #[error("upstream unavailable: {0}")]
    Unavailable(String),
    /// The call ran out of time.
    #[error("timed out")]
    Timeout,
    /// A response could not be decoded.
    #[error("cannot decode the response: {0}")]
    Decode(String),
    /// The daily cap on web calls for the project is spent.
    #[error("the daily web cap of {limit} calls is spent")]
    DailyCapSpent {
        /// The cap that was reached.
        limit: u32,
    },
    /// A URL is malformed or not fetchable.
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    /// A URL resolves to a private, loopback, or link-local address.
    #[error("{0} resolves to a non-public address")]
    PrivateAddress(String),
    /// The page is not something `web_read` returns text from.
    #[error("unsupported content: {0}")]
    UnsupportedContent(String),
    /// The page is larger than the read limit.
    #[error("the page is larger than {limit} bytes")]
    TooLarge {
        /// The limit in bytes.
        limit: u64,
    },
    /// The page does not exist (404 or 410).
    #[error("not found")]
    NotFound,
    /// The page refused us (401 or 403), or `robots.txt` disallows it.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// A redirect chain went on for more than five hops.
    #[error("more than five redirects")]
    TooManyRedirects,
    /// A redirect chain came back to a URL it had already visited.
    #[error("redirect loop")]
    RedirectLoop,
    /// The URL, or a redirect on the way to the page, is on arXiv, whose
    /// papers `lit_get_paper` reads from the LaTeX source instead.
    #[error("arXiv papers are read with lit_get_paper, which reads the LaTeX source")]
    ArxivPaper {
        /// The paper's id, without a version, when the URL names one.
        arxiv_id: Option<String>,
    },
    /// A redirect chain ended without a destination.
    #[error("redirect ended without a destination (status {0})")]
    DeadEnd(u16),
}

impl Error {
    /// The envelope error code this error reports as.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidUrl(_)
            | Self::PrivateAddress(_)
            | Self::UnsupportedContent(_)
            | Self::TooLarge { .. }
            | Self::ArxivPaper { .. } => ErrorCode::InvalidInput,
            Self::NotFound => ErrorCode::NotFound,
            Self::Forbidden(_) => ErrorCode::Forbidden,
            Self::RateLimited { .. } => ErrorCode::RateLimited,
            Self::Timeout => ErrorCode::Timeout,
            Self::DailyCapSpent { .. } => ErrorCode::BudgetExceeded,
            Self::InvalidConfig { .. }
            | Self::Client(_)
            | Self::MissingApiKey
            | Self::Unavailable(_)
            | Self::Decode(_)
            | Self::TooManyRedirects
            | Self::RedirectLoop
            | Self::DeadEnd(_) => ErrorCode::BackendUnavailable,
        }
    }

    /// The `error` envelope for this error, with `retry_after_ms` set for a
    /// rate limit and a `lit_get_paper` next step for an arXiv paper.
    #[must_use]
    pub fn into_envelope(self) -> Envelope {
        let envelope = Envelope::error(self.code(), self.to_string());
        match self {
            Self::RateLimited { retry_after_ms } => envelope.with_retry_after_ms(retry_after_ms),
            Self::ArxivPaper { arxiv_id: Some(id) } => {
                envelope.with_next([format!("lit_get_paper {{\"arxiv_id\": \"{id}\"}}")])
            }
            _ => envelope,
        }
    }
}

#[cfg(test)]
mod test;
