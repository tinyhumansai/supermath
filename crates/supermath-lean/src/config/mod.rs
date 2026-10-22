//! [`LeanConfig`]: where the Lean service and the hosted indexes are, and the
//! per-tool time budgets.

/// The environment variable naming the Lean service.
pub const LEAN_URL_ENV: &str = "LEAN_URL";
/// The environment variable overriding the Loogle endpoint.
pub const LOOGLE_URL_ENV: &str = "LOOGLE_URL";
/// The environment variable overriding the `LeanSearch` endpoint.
pub const LEANSEARCH_URL_ENV: &str = "LEANSEARCH_URL";

/// Loogle's JSON endpoint, used by `lean_search` in `pattern` mode.
pub const DEFAULT_LOOGLE_URL: &str = "https://loogle.lean-lang.org/json";
/// `LeanSearch`'s search endpoint, used by `lean_search` in `natural` mode.
pub const DEFAULT_LEANSEARCH_URL: &str = "https://leansearch.net/search";

/// Settings for the Lean client and tools.
///
/// With no [`LeanConfig::url`] every tool that needs the service answers
/// `BACKEND_UNAVAILABLE`; nothing is checked and nothing is guessed.
///
/// ```
/// use supermath_lean::LeanConfig;
///
/// let config = LeanConfig::from_lookup(|name| {
///     (name == "LEAN_URL").then(|| "http://lean:3150".to_owned())
/// });
/// assert_eq!(config.url.as_deref(), Some("http://lean:3150"));
/// assert_eq!(config.check_timeout_ms, 60_000);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanConfig {
    /// The Lean service root (`LEAN_URL`), for example `http://lean:3150`.
    /// `None` means no service is configured.
    pub url: Option<String>,
    /// The Loogle JSON endpoint (`LOOGLE_URL`).
    pub loogle_url: String,
    /// The `LeanSearch` endpoint (`LEANSEARCH_URL`).
    pub leansearch_url: String,
    /// Wall time for one `lean_check`, in milliseconds (60 s).
    pub check_timeout_ms: u64,
    /// Wall time for one tactic in `lean_try_tactics` (10 s).
    pub tactic_timeout_ms: u64,
    /// Wall time for a whole `lean_build` (15 minutes).
    pub build_timeout_ms: u64,
    /// Wall time for a Mathlib search or a declaration lookup (15 s).
    pub query_timeout_ms: u64,
    /// Extra time an HTTP request gets beyond the time it asks the service
    /// for, so the service's own timeout answers first.
    pub http_slack_ms: u64,
    /// The most values `lean_search_counterexample` admits in one range.
    pub max_range: u64,
}

impl Default for LeanConfig {
    fn default() -> Self {
        Self {
            url: None,
            loogle_url: DEFAULT_LOOGLE_URL.into(),
            leansearch_url: DEFAULT_LEANSEARCH_URL.into(),
            check_timeout_ms: 60_000,
            tactic_timeout_ms: 10_000,
            build_timeout_ms: 15 * 60_000,
            query_timeout_ms: 15_000,
            http_slack_ms: 10_000,
            max_range: 1_000_000,
        }
    }
}

impl LeanConfig {
    /// Reads the process environment; see [`LeanConfig::from_lookup`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads settings through `lookup`, which returns a variable's value.
    /// Unset or blank variables keep their defaults; a trailing `/` on
    /// `LEAN_URL` is dropped.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());
        let default = Self::default();
        Self {
            url: get(LEAN_URL_ENV).map(|u| u.trim().trim_end_matches('/').to_owned()),
            loogle_url: get(LOOGLE_URL_ENV).unwrap_or(default.loogle_url),
            leansearch_url: get(LEANSEARCH_URL_ENV).unwrap_or(default.leansearch_url),
            ..default
        }
    }
}

#[cfg(test)]
mod test;
