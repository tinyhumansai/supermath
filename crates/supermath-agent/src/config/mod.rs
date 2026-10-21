//! Run configuration, read from the environment.
//!
//! Two model endpoints, each with one job:
//!
//! - **The LLM ladder** (`LADDER_URL`, `LADDER_API_KEY`) serves every chat
//!   completion and every embedding. It is an OpenAI-compatible router that
//!   picks the best-priced model for a named ladder such as `reasoning`.
//! - **`OpenRouter`** (`OPENROUTER_API_KEY`) serves Jev only: the decision model
//!   `OpenHuman`'s `tool_search` uses to pick deferred tools.
//!
//! [`Config::from_env`] reads the process environment;
//! [`Config::from_lookup`] takes any lookup function, so tests never touch
//! the real environment. `docs/specs/solving-harness.md` lists every
//! variable.

use std::path::PathBuf;

use crate::error::{Error, Result};

/// The ladder's API root when none is configured.
pub const DEFAULT_LADDER_URL: &str = "http://localhost:6969/v1";

/// The ladder the solver's turns run on when none is configured.
pub const DEFAULT_MODEL: &str = "reasoning";

/// The ladder that serves embeddings when none is configured.
pub const DEFAULT_EMBEDDING_MODEL: &str = "vectors";

/// The width of [`DEFAULT_EMBEDDING_MODEL`]'s vectors.
pub const DEFAULT_EMBEDDING_DIMS: usize = 1024;

/// The `CortexDB` address used when none is configured.
pub const DEFAULT_CORTEX_URL: &str = "http://localhost:3141";

/// The scope subtree this deployment owns when none is configured.
pub const DEFAULT_SCOPE_ROOT: &str = "ws:supermath";

/// Where the `OpenHuman` workspace lives when none is configured.
pub const DEFAULT_WORKSPACE: &str = ".supermath";

/// Everything one run needs to reach its models and its memory.
#[derive(Clone)]
pub struct Config {
    /// The ladder's OpenAI-compatible API root.
    pub ladder_url: String,
    /// Bearer token the ladder requires.
    pub ladder_api_key: String,
    /// The ladder the solver's turns run on, such as `reasoning`.
    pub model: String,
    /// The ladder that serves embeddings.
    pub embedding_model: String,
    /// The width of [`Self::embedding_model`]'s vectors.
    pub embedding_dims: usize,
    /// `OpenRouter` key; Jev's decision calls are its only use.
    pub openrouter_api_key: String,
    /// `OpenHuman` backend URL; `None` leaves `OpenHuman`'s own default.
    pub backend_url: Option<String>,
    /// Directory holding the `OpenHuman` workspace and agent home.
    pub workspace: PathBuf,
    /// `CortexDB` base URL, without a trailing slash.
    pub cortex_url: String,
    /// `CortexDB` operator key, sent as a bearer token.
    pub cortex_api_key: String,
    /// The scope subtree every memory this deployment writes hangs below.
    pub scope_root: String,
}

impl std::fmt::Debug for Config {
    // Hand-written so a logged config never carries a credential.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("ladder_url", &self.ladder_url)
            .field("model", &self.model)
            .field("embedding_model", &self.embedding_model)
            .field("embedding_dims", &self.embedding_dims)
            .field("backend_url", &self.backend_url)
            .field("workspace", &self.workspace)
            .field("cortex_url", &self.cortex_url)
            .field("scope_root", &self.scope_root)
            .finish_non_exhaustive()
    }
}

impl Config {
    /// Reads the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingConfig`] naming the first required variable
    /// that is unset or blank, and [`Error::InvalidConfig`] when a numeric
    /// setting does not parse.
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
    /// that is unset or blank, and [`Error::InvalidConfig`] when a numeric
    /// setting does not parse.
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
        Ok(Self {
            ladder_url: or("LADDER_URL", DEFAULT_LADDER_URL)
                .trim_end_matches('/')
                .to_string(),
            ladder_api_key: required("LADDER_API_KEY")?,
            model: or("SUPERMATH_MODEL", DEFAULT_MODEL),
            embedding_model: or("SUPERMATH_EMBEDDING_MODEL", DEFAULT_EMBEDDING_MODEL),
            embedding_dims,
            openrouter_api_key: required("OPENROUTER_API_KEY")?,
            backend_url: get("SUPERMATH_BACKEND_URL"),
            workspace: PathBuf::from(or("SUPERMATH_WORKSPACE", DEFAULT_WORKSPACE)),
            cortex_url: or("CORTEX_API_URL", DEFAULT_CORTEX_URL)
                .trim_end_matches('/')
                .to_string(),
            cortex_api_key: required("CORTEX_API_KEY")?,
            scope_root: or("CORTEX_SCOPE_ROOT", DEFAULT_SCOPE_ROOT),
        })
    }
}

#[cfg(test)]
mod test;
