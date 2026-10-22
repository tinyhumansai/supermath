//! Crate-wide error and result types.
//!
//! These are the failures of *building* the sub-agents: a missing primitive, a
//! bad setting, or `OpenHuman` refusing a child. A tool call never returns one
//! of these; it answers an [`Envelope`](supermath_tool::Envelope).

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A setting was present but not a usable value.
    #[error("setting `{name}` has an invalid value `{value}`")]
    InvalidConfig {
        /// The variable that was read.
        name: &'static str,
        /// The value it held.
        value: String,
    },

    /// The reviewer was set to run on the formalizer's model, so the model
    /// that wrote a statement would judge it.
    #[error("the reviewer must not run on the formalizer's model `{0}`")]
    ReviewerIsFormalizer(String),

    /// `OpenHuman` refused to build a child agent.
    #[error("openhuman refused child `{id}`: {detail}")]
    Agent {
        /// The agent id that was refused.
        id: String,
        /// What `OpenHuman` said.
        detail: String,
    },

    /// The registry the composition handed over lacks a tool a child needs.
    #[error("tool registry lacks `{0}`")]
    MissingPrimitive(String),

    /// A tool a child is listed to call is itself agentic, which would let
    /// children spawn children.
    #[error("`{0}` is an agentic tool and cannot be a child's primitive")]
    NotPrimitive(String),

    /// The checked-in benchmark set does not parse.
    #[error("benchmark set is malformed: {0}")]
    Bench(String),

    /// Registering a built tool failed.
    #[error(transparent)]
    Registry(#[from] supermath_tool::Error),
}

/// The result type of this crate's fallible public functions.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test;
