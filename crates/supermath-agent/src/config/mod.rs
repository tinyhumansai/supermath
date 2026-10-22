//! Run configuration, read from the environment.
//!
//! Two model endpoints, each with one job:
//!
//! - **Surplus Intelligence** (`SURPLUS_URL`, `SURPLUS_API_KEY`) serves every
//!   chat completion and every embedding, called directly through its
//!   OpenAI-compatible API with a concrete model name such as
//!   `deepseek-v4.1-flash`.
//! - **`OpenRouter`** (`OPENROUTER_API_KEY`) serves Jev only: the decision model
//!   `OpenHuman`'s `tool_search` uses to pick deferred tools.
//!
//! [`Config::from_env`] reads the process environment;
//! [`Config::from_lookup`] takes any lookup function, so tests never touch
//! the real environment. `docs/specs/agent-runner.md` lists every
//! variable.

use std::path::PathBuf;

use supermath_subagents::Models;

use crate::error::{Error, Result};

/// Surplus Intelligence's API root when none is configured.
pub const DEFAULT_SURPLUS_URL: &str = "https://api.surplusintelligence.ai/v1";

/// The Surplus model the solver's turns run on when none is configured.
pub const DEFAULT_MODEL: &str = "deepseek-v4.1-flash";

/// How hard the solver's model thinks when nothing else is configured.
pub const DEFAULT_REASONING_EFFORT: &str = "medium";

/// The reasoning efforts `OpenHuman` accepts, aliases included.
pub const REASONING_EFFORTS: [&str; 8] = [
    "none", "minimal", "low", "medium", "high", "xhigh", "off", "max",
];

/// The Surplus model that serves embeddings when none is configured.
pub const DEFAULT_EMBEDDING_MODEL: &str = "venice-embed-1";

/// The width of [`DEFAULT_EMBEDDING_MODEL`]'s vectors.
pub const DEFAULT_EMBEDDING_DIMS: usize = 1024;

/// Where the `OpenHuman` workspace lives when none is configured.
pub const DEFAULT_WORKSPACE: &str = ".supermath";

/// Everything one run needs to reach its models and its memory.
#[derive(Clone)]
pub struct Config {
    /// Surplus's OpenAI-compatible API root.
    pub surplus_url: String,
    /// Bearer token Surplus requires.
    pub surplus_api_key: String,
    /// The Surplus model the solver's turns run on, such as
    /// `deepseek-v4.1-flash`.
    pub model: String,
    /// The reasoning effort every turn asks for, such as `medium`, sent with
    /// each request.
    pub reasoning_effort: String,
    /// The Surplus model that serves embeddings.
    pub embedding_model: String,
    /// The width of [`Self::embedding_model`]'s vectors.
    pub embedding_dims: usize,
    /// `OpenRouter` key; Jev's decision calls are its only use.
    pub openrouter_api_key: String,
    /// `OpenHuman` backend URL; `None` leaves `OpenHuman`'s own default.
    pub backend_url: Option<String>,
    /// Directory holding the `OpenHuman` workspace and agent home.
    pub workspace: PathBuf,
    /// Every tool domain's settings, including `CortexDB` (the ledger).
    pub toolkit: supermath_mcp::Config,
    /// The Surplus model each `agent_*` child runs on.
    pub models: Models,
    /// Command line of the Jacobian MCP server, when `SUPERMATH_JACOBIAN_CMD`
    /// is set.
    pub jacobian_cmd: Option<String>,
    /// Command line of the lean-lsp-mcp server, when `SUPERMATH_LEAN_LSP_CMD`
    /// is set.
    pub lean_lsp_cmd: Option<String>,
}

impl std::fmt::Debug for Config {
    // Hand-written so a logged config never carries a credential.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("surplus_url", &self.surplus_url)
            .field("model", &self.model)
            .field("reasoning_effort", &self.reasoning_effort)
            .field("embedding_model", &self.embedding_model)
            .field("embedding_dims", &self.embedding_dims)
            .field("backend_url", &self.backend_url)
            .field("workspace", &self.workspace)
            .field("cortex_url", &self.toolkit.ledger.cortex_url)
            .field("scope_root", &self.toolkit.ledger.scope_root)
            .field("models", &self.models)
            .field("jacobian_cmd", &self.jacobian_cmd)
            .field("lean_lsp_cmd", &self.lean_lsp_cmd)
            .finish_non_exhaustive()
    }
}

impl Config {
    /// Reads the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingConfig`] naming the first required variable
    /// that is unset or blank, [`Error::InvalidConfig`] when a numeric
    /// setting does not parse, and [`Error::Toolkit`] or [`Error::Subagents`]
    /// when a domain or child setting is invalid.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the configuration through `lookup`, which maps a variable name
    /// to its value.
    ///
    /// A blank value counts as unset, so `FOO=` in a `.env` file never
    /// becomes an empty credential.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingConfig`] naming the first required variable
    /// that is unset or blank, [`Error::InvalidConfig`] when a numeric
    /// setting does not parse, and [`Error::Toolkit`] or [`Error::Subagents`]
    /// when a domain or child setting is invalid.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let get = |name: &str| {
            lookup(name)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let required = |name: &'static str| get(name).ok_or(Error::MissingConfig(name));
        let or = |name: &str, default: &str| get(name).unwrap_or_else(|| default.to_string());
        let embedding_dims =
            match get("SUPERMATH_EMBEDDING_DIMS") {
                None => DEFAULT_EMBEDDING_DIMS,
                Some(raw) => raw.parse::<usize>().ok().filter(|dims| *dims > 0).ok_or(
                    Error::InvalidConfig {
                        name: "SUPERMATH_EMBEDDING_DIMS",
                        value: raw,
                    },
                )?,
            };
        let reasoning_effort = or("SUPERMATH_REASONING_EFFORT", DEFAULT_REASONING_EFFORT);
        if !REASONING_EFFORTS.contains(&reasoning_effort.as_str()) {
            return Err(Error::InvalidConfig {
                name: "SUPERMATH_REASONING_EFFORT",
                value: reasoning_effort,
            });
        }
        let surplus_api_key = required("SURPLUS_API_KEY")?;
        let openrouter_api_key = required("OPENROUTER_API_KEY")?;
        // The agent keeps memory: unlike the MCP server, it refuses to start
        // without the CortexDB key.
        required("CORTEX_API_KEY")?;
        let toolkit = supermath_mcp::Config::from_lookup(&lookup)?;
        Ok(Self {
            surplus_url: or("SURPLUS_URL", DEFAULT_SURPLUS_URL)
                .trim_end_matches('/')
                .to_string(),
            surplus_api_key,
            model: or("SUPERMATH_MODEL", DEFAULT_MODEL),
            reasoning_effort,
            embedding_model: or("SUPERMATH_EMBEDDING_MODEL", DEFAULT_EMBEDDING_MODEL),
            embedding_dims,
            openrouter_api_key,
            backend_url: get("SUPERMATH_BACKEND_URL"),
            workspace: PathBuf::from(or("SUPERMATH_WORKSPACE", DEFAULT_WORKSPACE)),
            toolkit,
            models: Models::from_lookup(&lookup)?,
            jacobian_cmd: get("SUPERMATH_JACOBIAN_CMD"),
            lean_lsp_cmd: get("SUPERMATH_LEAN_LSP_CMD"),
        })
    }
}

#[cfg(test)]
mod test;
