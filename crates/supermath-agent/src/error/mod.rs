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

    /// The toolkit could not be composed, for example because the project
    /// name has no usable characters.
    #[error(transparent)]
    Toolkit(#[from] supermath_mcp::Error),

    /// `CortexDB` is unreachable or not ready to store recallable memories.
    #[error(transparent)]
    Memory(#[from] supermath_ledger::Error),

    /// The `agent_*` tools could not be built or configured.
    #[error(transparent)]
    Subagents(#[from] supermath_subagents::Error),

    /// A tool was offered to the host adapter twice under one name.
    #[error("tool `{0}` is registered twice")]
    DuplicateTool(String),

    /// The `OpenHuman` runtime failed to boot or to register the agent.
    #[error("openhuman runtime failed: {0}")]
    Runtime(String),
}

/// The crate's standard result type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test;
