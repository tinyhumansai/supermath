//! [`PythonConfig`]: where the service is and the caps on a run.

/// The environment variable naming the service, e.g. `http://localhost:3160`.
pub const URL_ENV: &str = "PYTHON_URL";
/// The environment variable for the default run timeout in seconds.
pub const DEFAULT_TIMEOUT_ENV: &str = "SUPERMATH_PY_DEFAULT_TIMEOUT";
/// The environment variable for the longest run timeout in seconds.
pub const MAX_TIMEOUT_ENV: &str = "SUPERMATH_PY_MAX_TIMEOUT";
/// The environment variable for the largest counterexample frame.
pub const MAX_FRAME_ENV: &str = "SUPERMATH_PY_MAX_FRAME";

/// The default run timeout, in seconds.
pub const DEFAULT_TIMEOUT_S: u64 = 60;
/// The default cap on a run's timeout, in seconds.
pub const DEFAULT_MAX_TIMEOUT_S: u64 = 900;
/// The default cap on the number of values a counterexample search covers.
pub const DEFAULT_MAX_FRAME: u64 = 100_000_000;

/// Settings for the Python tools.
///
/// ```
/// use supermath_python::PythonConfig;
///
/// let config = PythonConfig::from_lookup(|name| {
///     (name == "PYTHON_URL").then(|| "http://localhost:3160".to_owned())
/// });
/// assert_eq!(config.url.as_deref(), Some("http://localhost:3160"));
/// assert_eq!(config.default_timeout_s, 60);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonConfig {
    /// The service's base URL (`PYTHON_URL`); `None` leaves every tool
    /// answering `BACKEND_UNAVAILABLE`.
    pub url: Option<String>,
    /// The timeout of a run that names none (`SUPERMATH_PY_DEFAULT_TIMEOUT`).
    pub default_timeout_s: u64,
    /// The longest timeout a run may ask for (`SUPERMATH_PY_MAX_TIMEOUT`).
    pub max_timeout_s: u64,
    /// The most values one counterexample search may cover
    /// (`SUPERMATH_PY_MAX_FRAME`).
    pub max_frame: u64,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            url: None,
            default_timeout_s: DEFAULT_TIMEOUT_S,
            max_timeout_s: DEFAULT_MAX_TIMEOUT_S,
            max_frame: DEFAULT_MAX_FRAME,
        }
    }
}

impl PythonConfig {
    /// Reads the process environment; see [`PythonConfig::from_lookup`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads settings through `lookup`, which returns a variable's value.
    /// Unset, blank, zero, or unparsable values keep their defaults, and the
    /// default timeout never exceeds the maximum.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let text = |name: &str| {
            lookup(name)
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };
        let number = |name: &str, default: u64| {
            text(name)
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(default)
        };
        let max_timeout_s = number(MAX_TIMEOUT_ENV, DEFAULT_MAX_TIMEOUT_S);
        Self {
            url: text(URL_ENV).map(|url| url.trim_end_matches('/').to_owned()),
            default_timeout_s: number(DEFAULT_TIMEOUT_ENV, DEFAULT_TIMEOUT_S).min(max_timeout_s),
            max_timeout_s,
            max_frame: number(MAX_FRAME_ENV, DEFAULT_MAX_FRAME),
        }
    }
}

#[cfg(test)]
mod test;
