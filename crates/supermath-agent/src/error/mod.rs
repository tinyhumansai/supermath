//! Crate-wide error and result types.
//!
//! Every fallible public function in this crate returns [`Result`], and every
//! failure mode is a distinct [`Error`] variant. Callers match on variants;
//! message text is not a stable API.

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A required environment variable was unset or blank.
    #[error("required setting `{0}` is not set")]
    MissingConfig(&'static str),

    /// A setting was present but not a valid value.
    #[error("setting `{name}` has an invalid value `{value}`")]
    InvalidConfig {
        /// The variable that was read.
        name: &'static str,
        /// The value it held.
        value: String,
    },

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

    /// The `OpenHuman` runtime failed to boot or to register the agent.
    #[error("openhuman runtime failed: {0}")]
    Runtime(String),
}

/// The crate's standard result type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test;
