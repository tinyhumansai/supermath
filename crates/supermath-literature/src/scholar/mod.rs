//! The Semantic Scholar client: references and citations of an arXiv paper.

mod types;

use std::sync::Arc;

use serde::Deserialize;
use url::Url;

use crate::cache::SEARCH_TTL_MS;
use crate::error::{Error, Result};
use crate::http::{Fetcher, Policy, age_secs};
use crate::{ArxivId, Clock, LiteratureConfig, SystemClock};

pub use types::{CitationPage, CitedPaper, Direction};

const SOURCE: &str = "semantic_scholar";
const DEFAULT_RETRY_MS: u64 = 1_000;
const FIELDS: &str = "title,year,venue,authors,externalIds,citationCount";

/// A client for Semantic Scholar's Graph API. The optional
/// `SEMANTIC_SCHOLAR_API_KEY` is sent as `x-api-key`.
#[derive(Debug)]
pub struct SemanticScholarClient {
    fetcher: Fetcher,
    base_url: String,
    api_key: Option<String>,
}

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    next: Option<u64>,
    #[serde(default)]
    data: Vec<Edge>,
}

#[derive(Deserialize)]
struct Edge {
    #[serde(rename = "citedPaper")]
    cited: Option<Raw>,
    #[serde(rename = "citingPaper")]
    citing: Option<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(rename = "paperId")]
    paper_id: Option<String>,
    title: Option<String>,
    year: Option<u32>,
    venue: Option<String>,
    #[serde(default)]
    authors: Vec<Author>,
    #[serde(rename = "externalIds")]
    external_ids: Option<ExternalIds>,
    #[serde(rename = "citationCount")]
    citation_count: Option<u64>,
}

#[derive(Deserialize)]
struct Author {
    name: Option<String>,
}

#[derive(Deserialize)]
struct ExternalIds {
    #[serde(rename = "ArXiv")]
    arxiv: Option<String>,
    #[serde(rename = "DOI")]
    doi: Option<String>,
}

impl From<Raw> for CitedPaper {
    fn from(raw: Raw) -> Self {
        let ids = raw.external_ids;
        CitedPaper {
            source: SOURCE,
            paper_id: raw.paper_id,
            arxiv_id: ids
                .as_ref()
                .and_then(|i| i.arxiv.as_deref())
                .map(unversioned),
            doi: ids.and_then(|i| i.doi),
            title: raw.title,
            year: raw.year,
            venue: raw.venue.filter(|v| !v.is_empty()),
            author_count: raw.authors.len(),
            authors: raw
                .authors
                .into_iter()
                .filter_map(|a| a.name)
                .take(3)
                .collect(),
            citation_count: raw.citation_count,
        }
    }
}

/// An arXiv id as rows carry it: without a version. Semantic Scholar
/// sometimes records `2310.01234v3`; an id that does not parse is kept as
/// given rather than dropped.
fn unversioned(raw: &str) -> String {
    ArxivId::parse(raw).map_or_else(|_| raw.to_owned(), |id| id.base().to_owned())
}

impl SemanticScholarClient {
    /// A client on the system clock.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] when the HTTP client cannot be built.
    pub fn new(config: &LiteratureConfig) -> Result<Self> {
        Self::with_clock(config, Arc::new(SystemClock))
    }

    /// A client on `clock`.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] when the HTTP client cannot be built.
    pub fn with_clock(config: &LiteratureConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        Ok(Self {
            fetcher: Fetcher::new(config, clock)?,
            base_url: config.scholar_url.trim_end_matches('/').to_owned(),
            api_key: config.scholar_api_key.clone(),
        })
    }

    /// Lists the references or citations of an arXiv paper, `limit` rows from
    /// `offset`.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] when Semantic Scholar does not know the paper,
    /// [`Error::RateLimited`] on a 429, and the other transport errors of
    /// [`ArxivClient::search`](crate::ArxivClient::search).
    pub async fn citations(
        &self,
        id: &ArxivId,
        direction: Direction,
        offset: u64,
        limit: usize,
    ) -> Result<CitationPage> {
        let edge = match direction {
            Direction::References => "references",
            Direction::CitedBy => "citations",
        };
        let mut url = Url::parse(&format!(
            "{}/paper/arXiv:{}/{edge}",
            self.base_url,
            id.base()
        ))
        .map_err(|e| Error::Unavailable {
            source_name: SOURCE.into(),
            reason: format!("bad base url: {e}"),
        })?;
        url.query_pairs_mut()
            .append_pair("fields", FIELDS)
            .append_pair("offset", &offset.to_string())
            .append_pair("limit", &limit.to_string());
        let policy = Policy {
            source: SOURCE,
            ttl_ms: Some(SEARCH_TTL_MS),
            limiter: None,
            headers: self
                .api_key
                .iter()
                .map(|key| ("x-api-key", key.clone()))
                .collect(),
            default_retry_ms: DEFAULT_RETRY_MS,
        };
        let body = self.fetcher.get(url.as_str(), &policy).await?;
        let page: Page = serde_json::from_slice(&body.bytes).map_err(|e| Error::Decode {
            what: "the semantic scholar answer".into(),
            reason: e.to_string(),
        })?;
        let papers = page
            .data
            .into_iter()
            .filter_map(|edge| match direction {
                Direction::References => edge.cited,
                Direction::CitedBy => edge.citing,
            })
            .map(CitedPaper::from)
            .collect();
        Ok(CitationPage {
            papers,
            next: page.next,
            cached_age_secs: age_secs(body.cached_age_ms),
        })
    }
}

#[cfg(test)]
mod test;
