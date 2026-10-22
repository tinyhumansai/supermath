//! Crate-wide error and result types.
//!
//! Every fallible public function in this crate returns [`Result`], and every
//! failure mode is a distinct [`Error`] variant. Tools turn an [`Error`] into
//! an `error` envelope through [`Error::code`]; message text is not a stable
//! API.

use supermath_tool::ErrorCode;

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A required environment variable was unset or blank.
    #[error("required setting `{0}` is not set")]
    MissingConfig(&'static str),

    /// A project name had no usable characters.
    #[error("project name `{0}` has no usable characters")]
    InvalidProject(String),

    /// `CortexDB` could not be reached, or did not answer in time.
    #[error("memory request `{operation}` failed: {detail}")]
    MemoryTransport {
        /// The memory operation that failed, such as `recall`.
        operation: &'static str,
        /// What went wrong, in the transport's words.
        detail: String,
    },

    /// `CortexDB` answered with a refusal.
    #[error("memory refused `{operation}` with status {status}: {detail}")]
    MemoryRefused {
        /// The memory operation that was refused.
        operation: &'static str,
        /// The HTTP status code.
        status: u16,
        /// The server's error code and message, when it sent them.
        detail: String,
    },

    /// `CortexDB` reported itself not ready to store recallable memories.
    #[error("memory is not ready: {0}")]
    MemoryNotReady(String),

    /// `CortexDB` stored a write without indexing it, so it cannot be recalled.
    #[error("memory stored the note but did not index it (completed: {completed})")]
    MemoryNotIndexed {
        /// The stages the server reported, comma-separated.
        completed: String,
    },

    /// The arguments failed to parse or broke a semantic rule.
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    /// A caller sent a status other than `asserted`; statuses are computed.
    #[error("status `{0}` cannot be set by a caller")]
    StatusFromCaller(String),

    /// A caller sent `evidence_origin: collected`; only the ledger sets it.
    #[error("evidence_origin `collected` cannot be set by a caller")]
    OriginFromCaller,

    /// A `tool` source named a call id this process never logged.
    #[error("call `{0}` is not in this process's call log")]
    UnrecordedCall(String),

    /// A logged call cannot back the claim.
    #[error("call `{call_id}` cannot back this claim: {reason}")]
    CallNotUsable {
        /// The call id the source named.
        call_id: String,
        /// Why the call does not qualify.
        reason: &'static str,
    },

    /// A `tool` source that could earn `checked` gave no frame.
    #[error("a tool source needs `frame`, the finite range the result covered")]
    MissingFrame,

    /// A `lean` source named a digest the file no longer has.
    #[error("{path} is at {current}, not {given}")]
    StaleDigest {
        /// The Lean file.
        path: String,
        /// The digest the caller supplied.
        given: String,
        /// The digest Lean checked.
        current: String,
    },

    /// A named claim is not in the ledger.
    #[error("claim `{0}` is not in the ledger")]
    ClaimNotFound(String),

    /// A `uses` edge would make a claim depend on itself.
    #[error("`{from}` uses `{to}`, which already reaches `{from}`")]
    LinkCycle {
        /// The claim that would gain the edge.
        from: String,
        /// The claim it would use.
        to: String,
    },

    /// The Lean evidence port found no such file or declaration.
    #[error("no declaration `{declaration}` in `{path}`")]
    EvidenceNotFound {
        /// The Lean file.
        path: String,
        /// The declaration.
        declaration: String,
    },

    /// The Lean evidence port could not reach Lean.
    #[error("lean evidence is unavailable: {0}")]
    EvidenceUnavailable(String),

    /// The Lean check ran out of time.
    #[error("the lean check timed out")]
    EvidenceTimeout,
}

/// The crate's standard result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The tool error code this error answers with.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::MissingConfig(_)
            | Self::MemoryTransport { .. }
            | Self::MemoryRefused { .. }
            | Self::MemoryNotReady(_)
            | Self::MemoryNotIndexed { .. }
            | Self::EvidenceUnavailable(_) => ErrorCode::BackendUnavailable,
            Self::InvalidProject(_)
            | Self::InvalidArguments(_)
            | Self::MissingFrame
            | Self::LinkCycle { .. } => ErrorCode::InvalidInput,
            Self::StatusFromCaller(_)
            | Self::OriginFromCaller
            | Self::UnrecordedCall(_)
            | Self::CallNotUsable { .. } => ErrorCode::Forbidden,
            Self::StaleDigest { .. } => ErrorCode::Conflict,
            Self::ClaimNotFound(_) | Self::EvidenceNotFound { .. } => ErrorCode::NotFound,
            Self::EvidenceTimeout => ErrorCode::Timeout,
        }
    }
}

#[cfg(test)]
mod test;
