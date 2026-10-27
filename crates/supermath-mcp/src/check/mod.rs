//! `supermath-mcp check`: is each backend the toolkit needs reachable?
//!
//! [`run`] probes the Lean toolchain and Mathlib, the Python version and
//! lock, `CortexDB`, Gemini, and arXiv, and returns a [`Report`].
//! A backend whose variable is unset is [`Status::Skipped`], not failed: the
//! toolkit runs without it and its tools answer `BACKEND_UNAVAILABLE`. A
//! configured backend that does not answer is [`Status::Failed`], and
//! [`Report::passed`] is then `false`.
//!
//! A Gemini key is probed with one ungrounded, one-token request, so a key
//! that is set but out of credit (402) or names a retired model (404) fails
//! the check. The key's value is never read into the report, and nothing
//! here prints a secret.

use std::fmt;

use supermath_literature::SearchQuery;

use crate::config::Config;
use crate::toolkit::Toolkit;

/// How one probe ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The backend answered.
    Ok,
    /// The backend is not configured.
    Skipped,
    /// The backend is configured and did not answer.
    Failed,
}

/// The result of one probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The backend, such as `lean`.
    pub name: &'static str,
    /// How the probe ended.
    pub status: Status,
    /// What it found, or why it failed.
    pub detail: String,
}

impl Line {
    fn new(name: &'static str, status: Status, detail: impl Into<String>) -> Self {
        Self {
            name,
            status,
            detail: detail.into(),
        }
    }

    fn from_result<T, E: fmt::Display>(
        name: &'static str,
        result: Result<T, E>,
        found: impl FnOnce(T) -> String,
    ) -> Self {
        match result {
            Ok(value) => Self::new(name, Status::Ok, found(value)),
            Err(error) => Self::new(name, Status::Failed, error.to_string()),
        }
    }
}

/// Every probe's result, in a fixed order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// One line per backend.
    pub lines: Vec<Line>,
}

impl Report {
    /// Whether no probe failed. Skipped backends do not count against it.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.lines.iter().all(|line| line.status != Status::Failed)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            let mark = match line.status {
                Status::Ok => "ok",
                Status::Skipped => "skipped",
                Status::Failed => "FAILED",
            };
            writeln!(f, "{:<7} {}: {}", mark, line.name, line.detail)?;
        }
        Ok(())
    }
}

/// Probes every backend of `toolkit`, which was composed from `config`.
pub async fn run(toolkit: &Toolkit, config: &Config) -> Report {
    let lean = if config.lean.url.is_some() {
        Line::from_result("lean", toolkit.lean().health().await, |health| {
            format!("toolchain {}, mathlib {}", health.toolchain, health.mathlib)
        })
    } else {
        Line::new("lean", Status::Skipped, "LEAN_URL is not set")
    };
    let python = if config.python.url.is_some() {
        Line::from_result(
            "python",
            toolkit.python().client().health().await,
            |health| {
                format!(
                    "python {}, lock {}",
                    health.python,
                    health.lock_sha256.as_deref().unwrap_or("unreported")
                )
            },
        )
    } else {
        Line::new("python", Status::Skipped, "PYTHON_URL is not set")
    };
    let cortex = if config.cortex_configured {
        Line::from_result("cortexdb", toolkit.memory().ready().await, |()| {
            format!("{} is ready", config.ledger.cortex_url)
        })
    } else {
        Line::new("cortexdb", Status::Skipped, "CORTEX_API_KEY is not set")
    };
    let gemini = if toolkit.has_gemini_key() {
        Line::from_result("gemini", toolkit.web().probe().await, |()| {
            format!("{} answered", config.web.model)
        })
    } else {
        Line::new(
            "gemini",
            Status::Skipped,
            "GEMINI_API_KEY is not set; web_search and web_research are unavailable",
        )
    };
    let query = SearchQuery {
        text: Some("prime".to_owned()),
        limit: 1,
        ..SearchQuery::default()
    };
    let arxiv = Line::from_result(
        "arxiv",
        toolkit.literature().arxiv().search(&query).await,
        |_| format!("{} answered", config.literature.arxiv_api_url),
    );
    Report {
        lines: vec![lean, python, cortex, gemini, arxiv],
    }
}

#[cfg(test)]
mod test;
