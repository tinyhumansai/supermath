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

    /// The problem file could not be read.
    #[error("cannot read problem file `{path}`: {source}")]
    ProblemFile {
        /// The path that was read.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The problem file held no statement.
    #[error("problem `{0}` has an empty statement")]
    EmptyProblem(String),

    /// The problem file name produced no usable slug.
    #[error("problem file `{0}` has no usable name")]
    UnnamedProblem(String),

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

    /// `CortexDB` stored a write without indexing it, so it cannot be recalled.
    #[error("memory stored the note but did not index it (completed: {completed})")]
    MemoryNotIndexed {
        /// The stages the server reported, comma-separated.
        completed: String,
    },

    /// The `OpenHuman` runtime failed to boot or to register the agent.
    #[error("openhuman runtime failed: {0}")]
    Runtime(String),

    /// The `OpenHuman` agent turn failed.
    #[error("agent turn failed: {0}")]
    Turn(String),
}

/// The crate's standard result type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test;
