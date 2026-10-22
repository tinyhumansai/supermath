//! The arXiv client: search, metadata, and LaTeX e-prints.
//!
//! Every request to arXiv waits on one [`RateLimiter`] shared by the client,
//! so the whole process keeps to one request per interval, and goes through
//! the disk cache first. Share one `Arc<ArxivClient>` between tools.

mod atom;
mod types;

use std::sync::Arc;

use url::Url;

use crate::cache::SEARCH_TTL_MS;
use crate::eprint::{Source, unpack};
use crate::error::{Error, Result};
use crate::http::{Body, Fetcher, Policy, age_secs};
use crate::{Clock, LiteratureConfig, RateLimiter, SystemClock};

pub use types::{ArxivId, Paper, Read, SearchPage, SearchQuery, SortOrder};

const SOURCE: &str = "arxiv";
const DEFAULT_RETRY_MS: u64 = 3_000;

/// A client for arXiv's Atom API and e-print endpoint.
///
/// ```no_run
/// # async fn demo() -> supermath_literature::Result<()> {
/// use supermath_literature::{ArxivClient, LiteratureConfig, SearchQuery};
///
/// let client = ArxivClient::new(&LiteratureConfig::from_env())?;
/// let page = client
///     .search(&SearchQuery {
///         text: Some("Sendov conjecture".into()),
///         limit: 5,
///         ..SearchQuery::default()
///     })
///     .await?;
/// println!("{} matches", page.total);
/// # Ok(()) }
/// ```
#[derive(Debug)]
pub struct ArxivClient {
    fetcher: Fetcher,
    limiter: RateLimiter,
    api_url: String,
    eprint_url: String,
}

impl ArxivClient {
    /// A client on the system clock.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] when the HTTP client cannot be built.
    pub fn new(config: &LiteratureConfig) -> Result<Self> {
        Self::with_clock(config, Arc::new(SystemClock))
    }

    /// A client on `clock`, which tests set to a [`ManualClock`](crate::ManualClock).
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] when the HTTP client cannot be built.
    pub fn with_clock(config: &LiteratureConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        Ok(Self {
            fetcher: Fetcher::new(config, clock.clone())?,
            limiter: RateLimiter::new(config.arxiv_interval_ms, clock),
            api_url: config.arxiv_api_url.clone(),
            eprint_url: config.arxiv_eprint_url.trim_end_matches('/').to_owned(),
        })
    }

    fn policy(&self, ttl_ms: Option<u64>) -> Policy<'_> {
        Policy {
            source: SOURCE,
            ttl_ms,
            limiter: Some(&self.limiter),
            headers: Vec::new(),
            default_retry_ms: DEFAULT_RETRY_MS,
        }
    }

    async fn feed(
        &self,
        params: &[(&str, String)],
        ttl_ms: Option<u64>,
    ) -> Result<(atom::Feed, Body)> {
        let mut url = Url::parse(&self.api_url).map_err(|e| Error::Unavailable {
            source_name: SOURCE.into(),
            reason: format!("bad api url: {e}"),
        })?;
        url.query_pairs_mut()
            .extend_pairs(params.iter().map(|(k, v)| (*k, v.as_str())));
        let body = self.fetcher.get(url.as_str(), &self.policy(ttl_ms)).await?;
        let xml = String::from_utf8_lossy(&body.bytes);
        Ok((atom::parse(&xml)?, body))
    }

    /// Searches arXiv.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] when the query names nothing to search for or
    /// `since` is not a `YYYY-MM-DD` date; [`Error::RateLimited`],
    /// [`Error::Timeout`], [`Error::Unavailable`], [`Error::Upstream`] or
    /// [`Error::Decode`] when arXiv cannot answer.
    pub async fn search(&self, query: &SearchQuery) -> Result<SearchPage> {
        let expression = search_expression(query)?;
        let params = [
            ("search_query", expression),
            ("start", query.start.to_string()),
            ("max_results", query.limit.max(1).to_string()),
            (
                "sortBy",
                match query.sort {
                    SortOrder::Relevance => "relevance",
                    SortOrder::Recent => "submittedDate",
                }
                .to_owned(),
            ),
            ("sortOrder", "descending".to_owned()),
        ];
        let (feed, body) = self.feed(&params, Some(SEARCH_TTL_MS)).await?;
        Ok(SearchPage {
            papers: feed.papers,
            total: feed.total,
            start: feed.start,
            cached_age_secs: age_secs(body.cached_age_ms),
        })
    }

    /// Fetches one paper's metadata. A versioned `id` is cached forever; an
    /// unversioned one resolves to the latest version and is cached for a day.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] when arXiv has no such paper, plus the errors of
    /// [`ArxivClient::search`].
    pub async fn paper(&self, id: &ArxivId) -> Result<(Paper, Option<u64>)> {
        let ttl = if id.version().is_some() {
            None
        } else {
            Some(SEARCH_TTL_MS)
        };
        let (feed, body) = self.feed(&[("id_list", id.to_string())], ttl).await?;
        let paper = feed
            .papers
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(format!("arxiv has no paper {id}")))?;
        Ok((paper, age_secs(body.cached_age_ms)))
    }

    /// Fetches and unpacks the e-print of `id`. The version must be named so
    /// the answer can be cached forever. A paper whose source arXiv does not
    /// serve (a 404) is reported as [`Source::PdfOnly`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] when `id` carries no version, [`Error::Decode`]
    /// for a corrupt archive, plus the transport errors of
    /// [`ArxivClient::search`].
    pub async fn eprint(&self, id: &ArxivId) -> Result<(Source, Option<u64>)> {
        if id.version().is_none() {
            return Err(Error::InvalidInput(format!(
                "e-print of {id} needs a version"
            )));
        }
        let url = format!("{}/{id}", self.eprint_url);
        match self.fetcher.get(&url, &self.policy(None)).await {
            Ok(body) => Ok((unpack(&body.bytes)?, age_secs(body.cached_age_ms))),
            Err(Error::NotFound(_)) => Ok((Source::PdfOnly, None)),
            Err(other) => Err(other),
        }
    }

    /// Fetches a paper's metadata and its source, pinned to the version the
    /// metadata names.
    ///
    /// # Errors
    ///
    /// The errors of [`ArxivClient::paper`] and [`ArxivClient::eprint`].
    pub async fn read(&self, id: &ArxivId) -> Result<Read> {
        let (paper, meta_age) = self.paper(id).await?;
        let pinned = id.with_version(paper.version);
        let (source, source_age) = self.eprint(&pinned).await?;
        Ok(Read {
            paper,
            source,
            cached_age_secs: meta_age.into_iter().chain(source_age).max(),
        })
    }
}

/// Builds arXiv's `search_query` expression: every clause joined by `AND`.
fn search_expression(query: &SearchQuery) -> Result<String> {
    let mut clauses = Vec::new();
    for (field, text) in [
        ("all", &query.text),
        ("ti", &query.title),
        ("au", &query.author),
    ] {
        if let Some(text) = text {
            clauses.extend(
                text.split_whitespace()
                    .map(|w| {
                        w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
                    })
                    .filter(|w| !w.is_empty())
                    .map(|w| format!("{field}:{w}")),
            );
        }
    }
    if clauses.is_empty() {
        return Err(Error::InvalidInput(
            "give a query, title, or author to search for".into(),
        ));
    }
    if let Some(category) = &query.category {
        let valid = !category.is_empty()
            && category
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
        if !valid {
            return Err(Error::InvalidInput(format!(
                "{category:?} is not an arxiv category"
            )));
        }
        clauses.push(format!("cat:{category}"));
    }
    if let Some(since) = &query.since {
        clauses.push(format!(
            "submittedDate:[{} TO 209912312359]",
            date_stamp(since)?
        ));
    }
    Ok(clauses.join(" AND "))
}

/// `2015-01-01` becomes `201501010000`.
fn date_stamp(date: &str) -> Result<String> {
    let parts: Vec<&str> = date.split('-').collect();
    let ok = matches!(parts.as_slice(), [y, m, d]
        if y.len() == 4 && m.len() == 2 && d.len() == 2
            && date.bytes().all(|b| b.is_ascii_digit() || b == b'-')
            && (1..=12).contains(&m.parse::<u32>().unwrap_or(0))
            && (1..=31).contains(&d.parse::<u32>().unwrap_or(0)));
    if ok {
        Ok(format!("{}0000", parts.concat()))
    } else {
        Err(Error::InvalidInput(format!(
            "since {date:?} is not a YYYY-MM-DD date"
        )))
    }
}

#[cfg(test)]
mod test;
