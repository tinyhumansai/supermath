//! The five `lit_*` tools and the [`Literature`] bundle that builds them.

mod citations;
mod extract_statements;
mod get_paper;
mod oeis;
mod search_arxiv;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;
use supermath_tool::{Coverage, Tool};

use crate::error::{Error, Result};
use crate::{ArxivClient, Clock, LiteratureConfig, OeisClient, SemanticScholarClient};

pub use citations::LitCitations;
pub use extract_statements::LitExtractStatements;
pub use get_paper::LitGetPaper;
pub use oeis::LitOeis;
pub use search_arxiv::LitSearchArxiv;

/// What a literature search never covers; named in every `coverage`.
pub(crate) const NOT_COVERED: [&str; 4] =
    ["zbmath", "mathscinet", "journals outside arxiv", "books"];

pub(crate) fn coverage(searched: &str) -> Coverage {
    Coverage::new([searched], NOT_COVERED)
}

pub(crate) fn cached(coverage: Coverage, age_secs: Option<u64>) -> Coverage {
    Coverage {
        cached_age: age_secs,
        ..coverage
    }
}

/// Parses tool arguments once, at the boundary.
pub(crate) fn parse_args<T: DeserializeOwned>(args: Value) -> Result<T> {
    serde_json::from_value(args).map_err(|e| Error::InvalidInput(e.to_string()))
}

/// A `cursor` argument: a decimal offset.
pub(crate) fn parse_cursor(cursor: Option<&str>) -> Result<u64> {
    cursor.map_or(Ok(0), |c| {
        c.parse()
            .map_err(|_| Error::InvalidInput(format!("cursor {c:?} is not one this tool returned")))
    })
}

/// Trims `text` to at most `max` characters, marking the cut with `…`.
pub(crate) fn trim_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((cut, _)) => format!("{}…", text[..cut].trim_end()),
        None => text.to_owned(),
    }
}

/// The clients behind the tools, built once and shared.
///
/// The arXiv client holds the process-wide rate limiter, so every tool that
/// talks to arXiv shares one `Arc<ArxivClient>`.
///
/// ```no_run
/// # fn demo() -> supermath_literature::Result<()> {
/// use supermath_literature::{Literature, LiteratureConfig};
///
/// let literature = Literature::new(&LiteratureConfig::from_env())?;
/// let names: Vec<_> = literature.tools().iter().map(|t| t.name()).collect();
/// assert_eq!(names.len(), 5);
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct Literature {
    arxiv: Arc<ArxivClient>,
    scholar: Arc<SemanticScholarClient>,
    oeis: Arc<OeisClient>,
}

impl Literature {
    /// Builds the clients on the system clock.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] when an HTTP client cannot be built.
    pub fn new(config: &LiteratureConfig) -> Result<Self> {
        Self::with_clock(config, Arc::new(crate::SystemClock))
    }

    /// Builds the clients on `clock`.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] when an HTTP client cannot be built.
    pub fn with_clock(config: &LiteratureConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        Ok(Self {
            arxiv: Arc::new(ArxivClient::with_clock(config, clock.clone())?),
            scholar: Arc::new(SemanticScholarClient::with_clock(config, clock.clone())?),
            oeis: Arc::new(OeisClient::with_clock(config, clock)?),
        })
    }

    /// The shared arXiv client.
    #[must_use]
    pub fn arxiv(&self) -> &Arc<ArxivClient> {
        &self.arxiv
    }

    /// All five tools, ready for a `ToolRegistry`.
    #[must_use]
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(LitSearchArxiv::new(self.arxiv.clone())),
            Arc::new(LitGetPaper::new(self.arxiv.clone())),
            Arc::new(LitExtractStatements::new(self.arxiv.clone())),
            Arc::new(LitCitations::new(self.scholar.clone())),
            Arc::new(LitOeis::new(self.oeis.clone())),
        ]
    }
}

#[cfg(test)]
mod test;
