//! The supermath agent: the embedded `OpenHuman` runtime, wired to Surplus
//! Intelligence, Jev, the composed toolkit, and the `agent_*` sub-agents.
//!
//! # Entry points
//!
//! - [`Config::from_env`] reads the Surplus, Jev, child-model, and toolkit
//!   settings.
//! - [`Runner::start`] boots `OpenHuman`; [`Runner::agent`] composes the
//!   toolkit for a project and registers the top-level agent as a [`Solver`].
//! - [`Host`] adapts the toolkit and the `agent_*` tools into `OpenHuman`
//!   tools ([`HostTool`]); every call goes through the toolkit's call log.
//!
//! # Example
//!
//! ```
//! use supermath_agent::Config;
//!
//! let config = Config::from_lookup(|name| match name {
//!     "SURPLUS_API_KEY" | "OPENROUTER_API_KEY" | "CORTEX_API_KEY" => Some("key".into()),
//!     "SUPERMATH_MODEL" => Some("deepseek-v4-pro".into()),
//!     _ => None,
//! })?;
//! assert_eq!(config.model, "deepseek-v4-pro");
//! assert_eq!(config.models.formalizer, "deepseek-v4-pro");
//! # Ok::<(), supermath_agent::Error>(())
//! ```
//!
//! # What this crate does not hold
//!
//! No tool, no agent loop, no provider client. Tools live in the domain
//! crates and are composed by `supermath-mcp`; the `agent_*` tools live in
//! `supermath-subagents`; the loop and providers are `OpenHuman`'s, and the
//! models are Surplus Intelligence's. This crate wires them and adapts the tool
//! contract to `OpenHuman`'s.

mod config;
mod error;
mod host;
mod runner;

pub use config::{
    Config, DEFAULT_EMBEDDING_DIMS, DEFAULT_EMBEDDING_MODEL, DEFAULT_MODEL,
    DEFAULT_REASONING_EFFORT, DEFAULT_SURPLUS_URL, DEFAULT_WORKSPACE, REASONING_EFFORTS,
};
pub use error::{Error, Result};
pub use host::{Host, HostTool};
pub use runner::{MAX_ITERATIONS, Runner, SYSTEM_PROMPT, Solver};
