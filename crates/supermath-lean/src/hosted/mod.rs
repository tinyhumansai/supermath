//! The hosted indexes behind `lean_search`'s `pattern` and `natural` modes.
//!
//! [Loogle](https://loogle.lean-lang.org) answers type-pattern queries and
//! [LeanSearch](https://leansearch.net) answers natural-language ones. Both
//! are public services on their own Mathlib revision, so the Lean service
//! (which has no network) never calls them; this crate does. Their replies
//! are read tolerantly: a row missing a field keeps an empty one.

use std::time::Duration;

use serde_json::{Value, json};

use crate::client::SearchRow;
use crate::config::LeanConfig;
use crate::error::{Error, Result};

/// Which hosted index to ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Index {
    /// Loogle: `(?a + ?b) ^ 2`.
    Loogle,
    /// `LeanSearch`: `sum of two squares`.
    LeanSearch,
}

impl Index {
    /// The index's name for errors and coverage.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Loogle => "loogle",
            Self::LeanSearch => "leansearch",
        }
    }
}

/// A client for the two hosted indexes.
#[derive(Debug, Clone)]
pub struct HostedSearch {
    http: reqwest::Client,
    loogle_url: String,
    leansearch_url: String,
    timeout: Duration,
}

impl HostedSearch {
    /// A client for the endpoints in `config`.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] if the HTTP client cannot be built.
    pub fn new(config: &LeanConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("supermath-lean/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| Error::unavailable("hosted search", format!("http client: {e}")))?;
        Ok(Self {
            http,
            loogle_url: config.loogle_url.clone(),
            leansearch_url: config.leansearch_url.clone(),
            timeout: Duration::from_millis(config.query_timeout_ms),
        })
    }

    /// Asks `index` for at most `limit` declarations matching `query`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] when Loogle rejects the query;
    /// [`Error::RateLimited`] on a 429 or 503; [`Error::Timeout`] when the
    /// index is too slow; [`Error::Unavailable`] otherwise.
    pub async fn search(&self, index: Index, query: &str, limit: usize) -> Result<Vec<SearchRow>> {
        let name = index.name();
        let request = match index {
            Index::Loogle => self.http.get(&self.loogle_url).query(&[("q", query)]),
            Index::LeanSearch => self
                .http
                .post(&self.leansearch_url)
                .json(&json!({"query": [query], "num_results": limit})),
        };
        let response = request
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| transport(name, &e))?;
        let status = response.status();
        if status.as_u16() == 429 || status.as_u16() == 503 {
            let retry_after_ms = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map_or(5_000, |secs| secs.saturating_mul(1000));
            return Err(Error::RateLimited {
                service: name.to_owned(),
                retry_after_ms,
            });
        }
        if !status.is_success() {
            return Err(Error::unavailable(
                name,
                format!("answered status {}", status.as_u16()),
            ));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| Error::unavailable(name, "unreadable reply"))?;
        let mut rows = match index {
            Index::Loogle => loogle_rows(&body)?,
            Index::LeanSearch => leansearch_rows(&body),
        };
        rows.truncate(limit);
        Ok(rows)
    }
}

fn transport(name: &str, error: &reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout(format!("{name} did not answer in time"))
    } else if error.is_connect() {
        Error::unavailable(name, "connection failed")
    } else {
        Error::unavailable(name, "request failed")
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn loogle_rows(body: &Value) -> Result<Vec<SearchRow>> {
    if let Some(message) = body.get("error").and_then(Value::as_str) {
        return Err(Error::InvalidInput(format!(
            "loogle rejected the query: {message}"
        )));
    }
    Ok(body
        .get("hits")
        .and_then(Value::as_array)
        .map(|hits| {
            hits.iter()
                .map(|hit| SearchRow {
                    name: text(hit, "name"),
                    ty: text(hit, "type"),
                    module: text(hit, "module"),
                })
                .collect()
        })
        .unwrap_or_default())
}

/// A dotted name from a JSON array of segments, or a plain string.
fn dotted(value: Option<&Value>) -> String {
    match value {
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("."),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn leansearch_rows(body: &Value) -> Vec<SearchRow> {
    // `[[{"result": {...}, "distance": ...}, ...]]`: one inner list per query.
    body.as_array()
        .and_then(|lists| lists.first())
        .and_then(Value::as_array)
        .map(|hits| {
            hits.iter()
                .filter_map(|hit| hit.get("result"))
                .map(|result| SearchRow {
                    name: dotted(result.get("name")),
                    ty: {
                        let ty = text(result, "type");
                        if ty.is_empty() {
                            text(result, "signature")
                        } else {
                            ty
                        }
                    },
                    module: dotted(result.get("module_name")),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod test;
