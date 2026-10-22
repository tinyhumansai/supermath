//! [`WebConfig`]: everything the web tools read from the environment.
//!
//! | Variable | Meaning | Default |
//! | --- | --- | --- |
//! | `GEMINI_API_KEY` | the Gemini API key; without it the Gemini tools answer `BACKEND_UNAVAILABLE` | unset |
//! | `SUPERMATH_WEB_MODEL` | the pinned Gemini model name | [`DEFAULT_MODEL`] |
//! | `SUPERMATH_WEB_DAILY_CAP` | `web_*` calls per project per day | 200 |
//! | `SUPERMATH_CACHE` | a directory; responses are cached in its `web` subdirectory | no cache |
//! | `SUPERMATH_CONTACT` | an address put in the `User-Agent` of page fetches | none |
//! | `SUPERMATH_WEB_BASE_URL` | the Gemini API root, for a proxy or a stub | Google's |

use std::path::PathBuf;
use std::time::Duration;

use crate::error::{Error, Result};

/// The Gemini model used when `SUPERMATH_WEB_MODEL` is unset. A pinned name,
/// never an alias that moves.
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";

/// The Gemini API root.
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// The default daily cap on `web_*` calls per project.
pub const DEFAULT_DAILY_CAP: u32 = 200;

/// Configuration of the web tools.
///
/// Build one with [`WebConfig::from_env`] or [`WebConfig::default`] and the
/// `with_*` methods. The API key is private and redacted from [`Debug`]
/// output.
#[derive(Clone)]
pub struct WebConfig {
    api_key: Option<String>,
    /// The Gemini API root, without a trailing slash.
    pub base_url: String,
    /// The Gemini model name.
    pub model: String,
    /// `web_*` calls allowed per project per UTC day.
    pub daily_cap: u32,
    /// Where responses are cached; `None` disables the cache.
    pub cache_dir: Option<PathBuf>,
    /// The contact address for the `User-Agent` of page fetches.
    pub contact: Option<String>,
    /// Wall time for one `web_search` call.
    pub search_timeout: Duration,
    /// Wall time for one `web_research` call.
    pub research_timeout: Duration,
    /// Wall time for one `web_read` call.
    pub read_timeout: Duration,
    /// Wall time for resolving one grounding redirect.
    pub redirect_timeout: Duration,
    /// Hosts whose URLs are grounding redirects, resolved by reading their
    /// `Location` header.
    pub redirect_hosts: Vec<String>,
    /// Lets `web_read` fetch private, loopback, and link-local addresses over
    /// plain `http`. Off by default; only tests and trusted networks set it.
    pub allow_private_hosts: bool,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.to_owned(),
            model: DEFAULT_MODEL.to_owned(),
            daily_cap: DEFAULT_DAILY_CAP,
            cache_dir: None,
            contact: None,
            search_timeout: Duration::from_secs(30),
            research_timeout: Duration::from_secs(180),
            read_timeout: Duration::from_secs(20),
            redirect_timeout: Duration::from_secs(5),
            redirect_hosts: vec!["vertexaisearch.cloud.google.com".to_owned()],
            allow_private_hosts: false,
        }
    }
}

impl std::fmt::Debug for WebConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("daily_cap", &self.daily_cap)
            .field("cache_dir", &self.cache_dir)
            .field("contact", &self.contact)
            .field("search_timeout", &self.search_timeout)
            .field("research_timeout", &self.research_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("redirect_timeout", &self.redirect_timeout)
            .field("redirect_hosts", &self.redirect_hosts)
            .field("allow_private_hosts", &self.allow_private_hosts)
            .finish()
    }
}

impl WebConfig {
    /// Reads the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] when `SUPERMATH_WEB_DAILY_CAP` is not a
    /// non-negative integer.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the configuration through `lookup`, which maps a variable name
    /// to its value. [`WebConfig::from_env`] passes the process environment.
    ///
    /// Empty values count as unset.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] when `SUPERMATH_WEB_DAILY_CAP` is not a
    /// non-negative integer.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());
        let mut config = Self {
            api_key: get("GEMINI_API_KEY"),
            cache_dir: get("SUPERMATH_CACHE").map(|dir| PathBuf::from(dir).join("web")),
            contact: get("SUPERMATH_CONTACT"),
            ..Self::default()
        };
        if let Some(model) = get("SUPERMATH_WEB_MODEL") {
            config.model = model;
        }
        if let Some(base) = get("SUPERMATH_WEB_BASE_URL") {
            base.trim_end_matches('/').clone_into(&mut config.base_url);
        }
        if let Some(cap) = get("SUPERMATH_WEB_DAILY_CAP") {
            config.daily_cap = cap.trim().parse().map_err(|_| Error::InvalidConfig {
                variable: "SUPERMATH_WEB_DAILY_CAP",
                reason: "expected a non-negative integer".to_owned(),
            })?;
        }
        Ok(config)
    }

    /// Sets the API key.
    #[must_use]
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Sets the Gemini API root.
    #[must_use]
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        base.into()
            .trim_end_matches('/')
            .clone_into(&mut self.base_url);
        self
    }

    /// Sets the response cache directory.
    #[must_use]
    pub fn with_cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }

    /// Whether a Gemini API key is configured.
    #[must_use]
    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    pub(crate) fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// The `User-Agent` of page fetches: names supermath and the contact.
    #[must_use]
    pub fn user_agent(&self) -> String {
        let version = env!("CARGO_PKG_VERSION");
        match &self.contact {
            Some(contact) => format!("supermath/{version} (+{contact})"),
            None => format!("supermath/{version}"),
        }
    }
}

#[cfg(test)]
mod test;
