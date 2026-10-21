//! Run configuration, read from the environment.
//!
//! [`Config::from_env`] reads the process environment;
//! [`Config::from_lookup`] takes any lookup function, so tests never touch
//! the real environment. `docs/specs/solving-harness.md` lists every
//! variable.

use std::path::PathBuf;

use crate::error::{Error, Result};

/// The OpenAI-compatible API root used when none is configured.
pub const DEFAULT_API_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// The `CortexDB` address used when none is configured.
pub const DEFAULT_CORTEX_URL: &str = "http://localhost:3141";

/// The scope subtree this deployment owns when none is configured.
pub const DEFAULT_SCOPE_ROOT: &str = "ws:supermath";

/// Where the `OpenHuman` workspace lives when none is configured.
pub const DEFAULT_WORKSPACE: &str = ".supermath";

/// Everything one run needs to reach its model and its memory.
#[derive(Clone)]
pub struct Config {
    /// OpenAI-compatible API root, such as `https://openrouter.ai/api/v1`.
    pub api_base_url: String,
    /// Bearer token for [`Self::api_base_url`].
    pub api_key: String,
    /// Model id sent with every request.
    pub model: String,
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
    // Hand-written so a logged config never carries either credential.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("api_base_url", &self.api_base_url)
            .field("model", &self.model)
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
    /// that is unset or blank.
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
    /// that is unset or blank.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let get = |name: &str| {
            lookup(name)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let required = |name: &'static str| get(name).ok_or(Error::MissingConfig(name));
        Ok(Self {
            api_base_url: get("SUPERMATH_API_BASE_URL")
                .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_string()),
            api_key: required("SUPERMATH_API_KEY")?,
            model: required("SUPERMATH_MODEL")?,
            backend_url: get("SUPERMATH_BACKEND_URL"),
            workspace: get("SUPERMATH_WORKSPACE")
                .map_or_else(|| PathBuf::from(DEFAULT_WORKSPACE), PathBuf::from),
            cortex_url: get("CORTEX_API_URL")
                .unwrap_or_else(|| DEFAULT_CORTEX_URL.to_string())
                .trim_end_matches('/')
                .to_string(),
            cortex_api_key: required("CORTEX_API_KEY")?,
            scope_root: get("CORTEX_SCOPE_ROOT").unwrap_or_else(|| DEFAULT_SCOPE_ROOT.to_string()),
        })
    }
}

#[cfg(test)]
mod test;
