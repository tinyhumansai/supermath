//! The OEIS client: sequences by terms or by words.

mod types;

use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::cache::SEARCH_TTL_MS;
use crate::error::{Error, Result};
use crate::http::{Body, Fetcher, Policy, age_secs};
use crate::{Clock, LiteratureConfig, SystemClock};

use types::{MAX_FORMULA_LINES, MAX_REFERENCE_LINES, MAX_TERMS};
pub use types::{OeisPage, Sequence};

const SOURCE: &str = "oeis";
const DEFAULT_RETRY_MS: u64 = 1_000;
/// Rows OEIS returns per search request; `start=` pages through the rest.
const PAGE_SIZE: usize = 10;

/// A client for `https://oeis.org/search?fmt=json`.
#[derive(Debug)]
pub struct OeisClient {
    fetcher: Fetcher,
    url: String,
}

#[derive(Deserialize)]
struct Wrapped {
    count: Option<u64>,
    results: Option<Vec<Raw>>,
}

#[derive(Deserialize)]
struct Raw {
    number: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    data: String,
    #[serde(default)]
    formula: Vec<String>,
    #[serde(default)]
    reference: Vec<String>,
}

impl From<Raw> for Sequence {
    fn from(raw: Raw) -> Self {
        Sequence {
            source: SOURCE,
            id: format!("A{:06}", raw.number),
            name: raw.name,
            terms: raw
                .data
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .take(MAX_TERMS)
                .map(|t| {
                    t.parse::<i64>()
                        .map_or_else(|_| Value::from(t), Value::from)
                })
                .collect(),
            formula: raw.formula.into_iter().take(MAX_FORMULA_LINES).collect(),
            references: raw
                .reference
                .into_iter()
                .take(MAX_REFERENCE_LINES)
                .collect(),
        }
    }
}

impl OeisClient {
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
            url: config.oeis_url.clone(),
        })
    }

    /// Searches OEIS for `query` (words, or comma-separated terms), `start`
    /// rows in, returning up to `limit` sequences.
    ///
    /// OEIS answers ten rows per request, so a `limit` above that
    /// fetches the following pages (`start + 10`, `start + 20`, …) until the
    /// limit is met or the matches run out. The page's `next` is the offset
    /// just past the last row shown, whichever request it came from.
    ///
    /// # Errors
    ///
    /// [`Error::RateLimited`], [`Error::Timeout`], [`Error::Unavailable`],
    /// [`Error::Upstream`] or [`Error::Decode`] when OEIS cannot answer any
    /// of the pages.
    pub async fn search(&self, query: &str, start: u64, limit: usize) -> Result<OeisPage> {
        let mut sequences: Vec<Sequence> = Vec::new();
        let mut total = None;
        let mut cached_age_ms: Option<u64> = None;
        let mut offset = start;
        while sequences.len() < limit {
            let body = self.fetch(query, offset).await?;
            cached_age_ms = cached_age_ms.max(body.cached_age_ms);
            let (raws, count) = parse(&body.bytes)?;
            total = count;
            let returned = raws.len();
            let wanted = limit - sequences.len();
            sequences.extend(raws.into_iter().take(wanted).map(Sequence::from));
            offset += returned as u64;
            let exhausted = total.is_some_and(|t| offset >= t);
            if returned < PAGE_SIZE || exhausted {
                break;
            }
        }
        let shown = sequences.len() as u64;
        let next = total
            .filter(|t| shown > 0 && start + shown < *t)
            .map(|_| start + shown);
        Ok(OeisPage {
            sequences,
            total,
            next,
            cached_age_secs: age_secs(cached_age_ms),
        })
    }

    /// One OEIS request: the page of matches beginning `start` rows in.
    async fn fetch(&self, query: &str, start: u64) -> Result<Body> {
        let mut url = Url::parse(&self.url).map_err(|e| Error::Unavailable {
            source_name: SOURCE.into(),
            reason: format!("bad url: {e}"),
        })?;
        url.query_pairs_mut()
            .append_pair("q", query)
            .append_pair("fmt", "json")
            .append_pair("start", &start.to_string());
        let policy = Policy {
            source: SOURCE,
            ttl_ms: Some(SEARCH_TTL_MS),
            limiter: None,
            headers: Vec::new(),
            default_retry_ms: DEFAULT_RETRY_MS,
        };
        self.fetcher.get(url.as_str(), &policy).await
    }
}

/// OEIS answers a bare array (older) or `{count, results}` (current), and
/// `null` results for no match.
fn parse(bytes: &[u8]) -> Result<(Vec<Raw>, Option<u64>)> {
    let decode = |e: serde_json::Error| Error::Decode {
        what: "the oeis answer".into(),
        reason: e.to_string(),
    };
    let value: Value = serde_json::from_slice(bytes).map_err(decode)?;
    match value {
        Value::Null => Ok((Vec::new(), Some(0))),
        Value::Array(_) => Ok((serde_json::from_value(value).map_err(decode)?, None)),
        _ => {
            let wrapped: Wrapped = serde_json::from_value(value).map_err(decode)?;
            Ok((wrapped.results.unwrap_or_default(), wrapped.count))
        }
    }
}

#[cfg(test)]
mod test;
