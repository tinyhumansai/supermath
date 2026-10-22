//! The envelope: one JSON shape for every tool answer.
//!
//! Build one with [`Envelope::ok`], [`Envelope::unknown`], or
//! [`Envelope::error`], then chain the `with_*` methods:
//!
//! ```
//! use supermath_tool::{Coverage, Envelope, Outcome};
//!
//! let envelope = Envelope::ok(serde_json::json!({"gcd": 6}))
//!     .with_coverage(Coverage::new(["arithmetic"], []));
//! assert_eq!(envelope.outcome, Outcome::Ok);
//! ```

mod types;

use serde_json::Value;

pub use types::{Coverage, Envelope, ErrorCode, ErrorInfo, Outcome, Usage};

/// Default number of rows in a list.
pub const DEFAULT_LIMIT: usize = 10;
/// Largest number of rows a list may return.
pub const MAX_LIMIT: usize = 50;
/// Most follow-up calls an envelope may suggest.
pub const MAX_NEXT: usize = 3;

/// Resolves a caller's requested row count to the capped value: the default
/// when absent, at least one, at most [`MAX_LIMIT`].
#[must_use]
pub fn clamp_limit(requested: Option<u64>) -> usize {
    requested.map_or(DEFAULT_LIMIT, |n| {
        usize::try_from(n).unwrap_or(MAX_LIMIT).clamp(1, MAX_LIMIT)
    })
}

impl Envelope {
    fn blank(outcome: Outcome) -> Self {
        Self {
            outcome,
            data: Value::Null,
            coverage: None,
            unknowns: Vec::new(),
            next: Vec::new(),
            cursor: None,
            usage: None,
            error: None,
        }
    }

    /// An `ok` envelope carrying `data`.
    #[must_use]
    pub fn ok(data: Value) -> Self {
        Self {
            data,
            ..Self::blank(Outcome::Ok)
        }
    }

    /// An `unknown` envelope: the answer could not be established, for the
    /// stated reason. Never a negative answer.
    #[must_use]
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            unknowns: vec![reason.into()],
            ..Self::blank(Outcome::Unknown)
        }
    }

    /// An `error` envelope. An error always has a code, so this is the only
    /// way to build one.
    #[must_use]
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: Some(ErrorInfo {
                code,
                message: message.into(),
                retry_after_ms: None,
            }),
            ..Self::blank(Outcome::Error)
        }
    }

    /// Replaces the payload.
    #[must_use]
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    /// Sets what the search covered.
    #[must_use]
    pub fn with_coverage(mut self, coverage: Coverage) -> Self {
        self.coverage = Some(coverage);
        self
    }

    /// Adds one thing the answer could not establish.
    #[must_use]
    pub fn with_unknown(mut self, unknown: impl Into<String>) -> Self {
        self.unknowns.push(unknown.into());
        self
    }

    /// Sets the suggested follow-up calls, keeping only the first
    /// [`MAX_NEXT`].
    #[must_use]
    pub fn with_next<S: Into<String>>(mut self, next: impl IntoIterator<Item = S>) -> Self {
        self.next = next.into_iter().take(MAX_NEXT).map(Into::into).collect();
        self
    }

    /// Sets the cursor for the next page.
    #[must_use]
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    /// Sets what an agentic tool spent.
    #[must_use]
    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }

    /// For a `RATE_LIMITED` error: when to retry. No effect on other
    /// envelopes.
    #[must_use]
    pub fn with_retry_after_ms(mut self, ms: u64) -> Self {
        if let Some(info) = self.error.as_mut() {
            info.retry_after_ms = Some(ms);
        }
        self
    }

    /// The error code, if this is an `error` envelope.
    #[must_use]
    pub fn code(&self) -> Option<ErrorCode> {
        self.error.as_ref().map(|info| info.code)
    }

    /// Whether the outcome is [`Outcome::Ok`].
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.outcome == Outcome::Ok
    }

    /// Whether the envelope is well formed: `error` is present exactly when
    /// the outcome is `error`, and `next` holds at most [`MAX_NEXT`] calls.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        (self.outcome == Outcome::Error) == self.error.is_some() && self.next.len() <= MAX_NEXT
    }
}

#[cfg(test)]
mod test;
