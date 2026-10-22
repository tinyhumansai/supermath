//! [`Config`]: the union of every domain's environment, read once.
//!
//! | Domain | Variables |
//! | --- | --- |
//! | files | `SUPERMATH_PROJECTS` |
//! | lean | `LEAN_URL`, `LOOGLE_URL`, `LEANSEARCH_URL` |
//! | python | `PYTHON_URL`, `SUPERMATH_PY_DEFAULT_TIMEOUT`, `SUPERMATH_PY_MAX_TIMEOUT`, `SUPERMATH_PY_MAX_FRAME` |
//! | literature | `SUPERMATH_CONTACT`, `SEMANTIC_SCHOLAR_API_KEY`, `SUPERMATH_CACHE`, and the base-URL overrides |
//! | web | `GEMINI_API_KEY`, `SUPERMATH_WEB_MODEL`, `SUPERMATH_WEB_DAILY_CAP`, `SUPERMATH_CACHE` |
//! | ledger | `CORTEX_API_URL`, `CORTEX_API_KEY`, `CORTEX_SCOPE_ROOT` |
//!
//! Every field is public and belongs to the domain that owns it, so a host
//! can read the environment and then override one domain, as the tests do.
//! A service whose variable is unset stays unconfigured: its tools still
//! register and answer `BACKEND_UNAVAILABLE`.

use supermath_files::FilesConfig;
use supermath_lean::LeanConfig;
use supermath_ledger::Config as LedgerConfig;
use supermath_literature::LiteratureConfig;
use supermath_python::PythonConfig;
use supermath_web::WebConfig;

use crate::error::Result;

/// The bearer token handed to the memory client when `CORTEX_API_KEY` is
/// unset. `CortexDB` refuses it, so every ledger call answers
/// `BACKEND_UNAVAILABLE` instead of the server failing to start.
const UNSET_CORTEX_KEY: &str = "unset";

/// Every domain's configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Where the project workspaces live.
    pub files: FilesConfig,
    /// The Lean service and the hosted indexes.
    pub lean: LeanConfig,
    /// The Python solver service.
    pub python: PythonConfig,
    /// arXiv, Semantic Scholar, and OEIS.
    pub literature: LiteratureConfig,
    /// Gemini grounded search and page reading.
    pub web: WebConfig,
    /// `CortexDB`, where the ledger lives.
    pub ledger: LedgerConfig,
    /// Whether `CORTEX_API_KEY` was set. When it was not, [`Config::ledger`]
    /// holds a placeholder key the server will refuse.
    pub cortex_configured: bool,
}

impl Config {
    /// Reads the process environment.
    ///
    /// # Errors
    ///
    /// [`Error::Web`](crate::Error::Web) when `SUPERMATH_WEB_DAILY_CAP` is
    /// not a non-negative integer; [`Error::Ledger`](crate::Error::Ledger) is
    /// not expected, because a missing key is replaced by a placeholder.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the configuration through `lookup`, so tests never touch the
    /// real environment.
    ///
    /// # Errors
    ///
    /// As [`Config::from_env`].
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let (ledger, cortex_configured) = if let Ok(ledger) = LedgerConfig::from_lookup(&lookup) {
            (ledger, true)
        } else {
            // Only a missing key fails; read the rest with a placeholder.
            let with_placeholder = |name: &str| {
                if name == "CORTEX_API_KEY" {
                    Some(UNSET_CORTEX_KEY.to_owned())
                } else {
                    lookup(name)
                }
            };
            (LedgerConfig::from_lookup(with_placeholder)?, false)
        };
        Ok(Self {
            files: FilesConfig::from_lookup(&lookup),
            lean: LeanConfig::from_lookup(&lookup),
            python: PythonConfig::from_lookup(&lookup),
            literature: LiteratureConfig::from_lookup(&lookup),
            web: WebConfig::from_lookup(&lookup)?,
            ledger,
            cortex_configured,
        })
    }
}

#[cfg(test)]
mod test;
