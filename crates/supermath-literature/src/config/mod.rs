//! [`LiteratureConfig`]: base URLs, credentials, and politeness settings.
//!
//! Every base URL is configurable so tests point the clients at loopback stubs
//! serving recorded fixtures; production uses the defaults.

use std::path::PathBuf;

/// arXiv's query API.
pub const DEFAULT_ARXIV_API_URL: &str = "https://export.arxiv.org/api/query";
/// arXiv's e-print (LaTeX source) endpoint.
pub const DEFAULT_ARXIV_EPRINT_URL: &str = "https://arxiv.org/e-print";
/// Semantic Scholar's Graph API.
pub const DEFAULT_SCHOLAR_URL: &str = "https://api.semanticscholar.org/graph/v1";
/// OEIS's search endpoint.
pub const DEFAULT_OEIS_URL: &str = "https://oeis.org/search";

/// Settings for the arXiv, Semantic Scholar and OEIS clients.
///
/// Build it from the environment with [`LiteratureConfig::from_env`], or from
/// [`Default`] and struct update syntax:
///
/// ```
/// use supermath_literature::LiteratureConfig;
///
/// let config = LiteratureConfig {
///     arxiv_interval_ms: 0,
///     ..LiteratureConfig::default()
/// };
/// assert_eq!(config.arxiv_interval_ms, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteratureConfig {
    /// The arXiv query API (`SUPERMATH_ARXIV_API_URL`).
    pub arxiv_api_url: String,
    /// The arXiv e-print endpoint (`SUPERMATH_ARXIV_EPRINT_URL`).
    pub arxiv_eprint_url: String,
    /// The Semantic Scholar Graph API root (`SUPERMATH_SCHOLAR_URL`).
    pub scholar_url: String,
    /// The OEIS search endpoint (`SUPERMATH_OEIS_URL`).
    pub oeis_url: String,
    /// A contact address sent in the `User-Agent` (`SUPERMATH_CONTACT`).
    pub contact: String,
    /// The optional Semantic Scholar key (`SEMANTIC_SCHOLAR_API_KEY`).
    pub scholar_api_key: Option<String>,
    /// Where responses are cached (`SUPERMATH_CACHE`); `None` disables it.
    pub cache_dir: Option<PathBuf>,
    /// The minimum gap between arXiv requests, in milliseconds.
    pub arxiv_interval_ms: u64,
    /// How long one HTTP request may take, in milliseconds.
    pub timeout_ms: u64,
}

impl Default for LiteratureConfig {
    fn default() -> Self {
        Self {
            arxiv_api_url: DEFAULT_ARXIV_API_URL.into(),
            arxiv_eprint_url: DEFAULT_ARXIV_EPRINT_URL.into(),
            scholar_url: DEFAULT_SCHOLAR_URL.into(),
            oeis_url: DEFAULT_OEIS_URL.into(),
            contact: "unset".into(),
            scholar_api_key: None,
            cache_dir: None,
            arxiv_interval_ms: 3_000,
            timeout_ms: 30_000,
        }
    }
}

impl LiteratureConfig {
    /// Reads the process environment; see [`LiteratureConfig::from_lookup`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads settings through `lookup`, which returns a variable's value.
    /// Unset or blank variables keep their defaults.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());
        let default = Self::default();
        Self {
            arxiv_api_url: get("SUPERMATH_ARXIV_API_URL").unwrap_or(default.arxiv_api_url),
            arxiv_eprint_url: get("SUPERMATH_ARXIV_EPRINT_URL").unwrap_or(default.arxiv_eprint_url),
            scholar_url: get("SUPERMATH_SCHOLAR_URL").unwrap_or(default.scholar_url),
            oeis_url: get("SUPERMATH_OEIS_URL").unwrap_or(default.oeis_url),
            contact: get("SUPERMATH_CONTACT").unwrap_or(default.contact),
            scholar_api_key: get("SEMANTIC_SCHOLAR_API_KEY"),
            cache_dir: get("SUPERMATH_CACHE").map(|dir| PathBuf::from(dir).join("literature")),
            ..default
        }
    }

    /// The `User-Agent` every request sends, naming supermath and a contact.
    #[must_use]
    pub fn user_agent(&self) -> String {
        format!(
            "supermath-literature/{} (mailto:{})",
            env!("CARGO_PKG_VERSION"),
            self.contact
        )
    }
}

#[cfg(test)]
mod test;
