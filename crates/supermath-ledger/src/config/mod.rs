//! Ledger configuration, read from the environment.
//!
//! | Variable | Default | Meaning |
//! | --- | --- | --- |
//! | `CORTEX_API_URL` | `http://localhost:3141` | the `CortexDB` base URL |
//! | `CORTEX_API_KEY` | required | the operator key, sent as a bearer token |
//! | `CORTEX_SCOPE_ROOT` | `ws:supermath` | the scope subtree this deployment owns |
//!
//! [`Config::from_env`] reads the process environment; [`Config::from_lookup`]
//! takes any lookup function, so tests never touch the real environment.

use crate::error::{Error, Result};
use crate::memory::Memory;

/// The `CortexDB` address used when none is configured.
pub const DEFAULT_CORTEX_URL: &str = "http://localhost:3141";

/// The scope subtree used when none is configured.
pub const DEFAULT_SCOPE_ROOT: &str = "ws:supermath";

/// Where the ledger's memory lives.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    /// `CortexDB` base URL, without a trailing slash.
    pub cortex_url: String,
    /// `CortexDB` operator key.
    pub cortex_api_key: String,
    /// The scope subtree every memory hangs below.
    pub scope_root: String,
}

impl std::fmt::Debug for Config {
    // Hand-written so a logged config never carries the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("cortex_url", &self.cortex_url)
            .field("cortex_api_key", &"<redacted>")
            .field("scope_root", &self.scope_root)
            .finish()
    }
}

impl Config {
    /// Reads the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// [`Error::MissingConfig`] when `CORTEX_API_KEY` is unset or blank.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the configuration through `lookup`.
    ///
    /// # Errors
    ///
    /// [`Error::MissingConfig`] when `CORTEX_API_KEY` is unset or blank.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let get = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Ok(Self {
            cortex_url: get("CORTEX_API_URL")
                .unwrap_or_else(|| DEFAULT_CORTEX_URL.to_string())
                .trim_end_matches('/')
                .to_string(),
            cortex_api_key: get("CORTEX_API_KEY").ok_or(Error::MissingConfig("CORTEX_API_KEY"))?,
            scope_root: get("CORTEX_SCOPE_ROOT").unwrap_or_else(|| DEFAULT_SCOPE_ROOT.to_string()),
        })
    }

    /// The `CortexDB` client this configuration describes.
    #[must_use]
    pub fn memory(&self) -> Memory {
        Memory::new(&self.cortex_url, &self.cortex_api_key, &self.scope_root)
    }
}

#[cfg(test)]
mod test;
