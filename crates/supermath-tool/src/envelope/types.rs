//! The envelope and the types it carries.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The stable error codes. Tests assert codes, never message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ErrorCode {
    /// Arguments failed the schema or a semantic check.
    InvalidInput,
    /// The named paper, declaration, or claim does not exist.
    NotFound,
    /// A service is down; no fallback was tried.
    BackendUnavailable,
    /// Admission refused the call before running it.
    BudgetExceeded,
    /// The call ran out of time; the result is unknown.
    Timeout,
    /// An upstream asked us to slow down; see `retry_after_ms`.
    RateLimited,
    /// The caller may not make this transition.
    Forbidden,
    /// A file changed since the caller last saw it.
    Conflict,
}

impl ErrorCode {
    /// The wire spelling, e.g. `"BACKEND_UNAVAILABLE"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::NotFound => "NOT_FOUND",
            Self::BackendUnavailable => "BACKEND_UNAVAILABLE",
            Self::BudgetExceeded => "BUDGET_EXCEEDED",
            Self::Timeout => "TIMEOUT",
            Self::RateLimited => "RATE_LIMITED",
            Self::Forbidden => "FORBIDDEN",
            Self::Conflict => "CONFLICT",
        }
    }
}

/// How a call ended. `Unknown` is never a negative answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The tool established its answer.
    Ok,
    /// The tool could not establish an answer; says nothing about the
    /// mathematics.
    Unknown,
    /// The call failed; the envelope's `error` holds the code.
    Error,
}

/// What a search covered. Absence of a result outside coverage is unknown.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    /// Sources that were searched.
    #[serde(default)]
    pub searched: Vec<String>,
    /// Sources that were not.
    #[serde(default)]
    pub not_covered: Vec<String>,
    /// Age in seconds of cached data the answer rests on, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_age: Option<u64>,
}

impl Coverage {
    /// Coverage that searched `searched` and left `not_covered` unsearched.
    #[must_use]
    pub fn new<S: Into<String>>(
        searched: impl IntoIterator<Item = S>,
        not_covered: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            searched: searched.into_iter().map(Into::into).collect(),
            not_covered: not_covered.into_iter().map(Into::into).collect(),
            cached_age: None,
        }
    }
}

/// What an agentic tool spent. Required on every agentic envelope.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// The model that ran the turn.
    pub model: String,
    /// Model turns taken.
    pub turns: u32,
    /// Tool calls the turn made.
    pub tool_calls: u32,
    /// Prompt tokens.
    pub input_tokens: u64,
    /// Completion tokens.
    pub output_tokens: u64,
    /// Wall time in milliseconds.
    pub elapsed_ms: u64,
}

/// The failure an `error` envelope carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// The stable code.
    pub code: ErrorCode,
    /// A one-line human message; never asserted on in tests.
    pub message: String,
    /// For `RATE_LIMITED`: when to retry, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

/// The one shape every tool answers with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    /// How the call ended.
    pub outcome: Outcome,
    /// The tool's payload; `null` when there is none.
    #[serde(default)]
    pub data: Value,
    /// What a search covered; present on every search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Coverage>,
    /// Anything the answer could not establish.
    #[serde(default)]
    pub unknowns: Vec<String>,
    /// At most three suggested follow-up calls, with arguments.
    #[serde(default)]
    pub next: Vec<String>,
    /// Pages a list; `null` at the end.
    #[serde(default)]
    pub cursor: Option<String>,
    /// What an agentic tool spent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Present exactly when `outcome` is `error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}
