//! [`LeanClient`]: the HTTP client for the Lean service.
//!
//! The service is described in `docs/specs/toolkit/lean/README.md`. Every
//! operation is a JSON `POST` to a fixed route and answers `200` with a
//! `status` field; `404` means the project file does not exist, `400` that
//! the request was malformed. Without a configured URL every method answers
//! [`Error::Unavailable`], so a stack with no Lean service says so instead of
//! guessing.

mod types;

use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::config::LeanConfig;
use crate::error::{Error, Result};

pub use types::{
    CheckReply, CheckRequest, CounterReply, CounterRequest, DeclReply, DeclRequest, GoalsReply,
    GoalsRequest, Health, Message, SearchReply, SearchRequest, SearchRow, Sorry, Status,
    TacticResult, TacticsReply, TacticsRequest,
};

/// A client for the Lean service at [`LeanConfig::url`].
///
/// ```
/// use supermath_lean::{LeanClient, LeanConfig};
///
/// let client = LeanClient::new(&LeanConfig::default()).unwrap();
/// assert!(!client.is_configured());
/// ```
#[derive(Debug, Clone)]
pub struct LeanClient {
    http: reqwest::Client,
    base: Option<String>,
    slack_ms: u64,
}

impl LeanClient {
    /// A client for the service in `config`.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] if the HTTP client cannot be built.
    pub fn new(config: &LeanConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| Error::unavailable("lean", format!("http client: {e}")))?;
        Ok(Self {
            http,
            base: config.url.clone(),
            slack_ms: config.http_slack_ms,
        })
    }

    /// Whether a service URL is configured.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.base.is_some()
    }

    fn base(&self) -> Result<&str> {
        self.base
            .as_deref()
            .ok_or_else(|| Error::unavailable("lean", "LEAN_URL is not set"))
    }

    /// The toolchain and Mathlib revision the service runs.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] when no URL is set, the service cannot be
    /// reached, or it answers with an error status; [`Error::Timeout`] if it
    /// does not answer in time.
    pub async fn health(&self) -> Result<Health> {
        let url = format!("{}/health", self.base()?);
        let request = self
            .http
            .get(url)
            .timeout(Duration::from_millis(self.slack_ms.max(1_000)));
        Self::decode(request.send().await).await
    }

    /// Elaborates a project file: `POST /check`.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] when the service has no such file;
    /// [`Error::Unavailable`] and [`Error::Timeout`] as for
    /// [`LeanClient::health`].
    pub async fn check(&self, request: &CheckRequest) -> Result<CheckReply> {
        self.post("check", request, request.timeout_ms).await
    }

    /// The goals at a position: `POST /goals`.
    ///
    /// # Errors
    ///
    /// As [`LeanClient::check`].
    pub async fn goals(&self, request: &GoalsRequest) -> Result<GoalsReply> {
        self.post("goals", request, request.timeout_ms).await
    }

    /// Tries tactics against a `sorry`: `POST /tactics`.
    ///
    /// # Errors
    ///
    /// As [`LeanClient::check`].
    pub async fn tactics(&self, request: &TacticsRequest) -> Result<TacticsReply> {
        let total = request
            .timeout_ms
            .saturating_mul(request.tactics.len().max(1) as u64);
        self.post("tactics", request, total).await
    }

    /// Looks a declaration up: `POST /decl`.
    ///
    /// # Errors
    ///
    /// As [`LeanClient::check`].
    pub async fn decl(&self, request: &DeclRequest) -> Result<DeclReply> {
        self.post("decl", request, request.timeout_ms).await
    }

    /// Searches Mathlib by name fragment: `POST /search`.
    ///
    /// # Errors
    ///
    /// As [`LeanClient::check`].
    pub async fn search(&self, request: &SearchRequest) -> Result<SearchReply> {
        self.post("search", request, request.timeout_ms).await
    }

    /// Searches a range for a witness: `POST /counterexample`.
    ///
    /// # Errors
    ///
    /// As [`LeanClient::check`].
    pub async fn counterexample(&self, request: &CounterRequest) -> Result<CounterReply> {
        self.post("counterexample", request, request.timeout_ms)
            .await
    }

    async fn post<Q: Serialize + Sync, R: DeserializeOwned>(
        &self,
        route: &str,
        body: &Q,
        timeout_ms: u64,
    ) -> Result<R> {
        let url = format!("{}/{route}", self.base()?);
        let request = self
            .http
            .post(url)
            .timeout(Duration::from_millis(
                timeout_ms.saturating_add(self.slack_ms),
            ))
            .json(body);
        Self::decode(request.send().await).await
    }

    async fn decode<R: DeserializeOwned>(
        sent: std::result::Result<reqwest::Response, reqwest::Error>,
    ) -> Result<R> {
        let response = sent.map_err(|e| transport(&e))?;
        let status = response.status();
        if status.is_success() {
            return response
                .json::<R>()
                .await
                .map_err(|e| transport_or(&e, "the service sent an unreadable reply"));
        }
        let message = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str().map(str::to_owned)))
            .unwrap_or_else(|| format!("status {}", status.as_u16()));
        Err(match status.as_u16() {
            404 => Error::NotFound(message),
            400 => Error::InvalidInput(message),
            code => Error::unavailable("lean", format!("answered status {code}: {message}")),
        })
    }
}

fn transport(error: &reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout("the lean service did not answer in time".into())
    } else if error.is_connect() {
        Error::unavailable("lean", "connection failed")
    } else {
        Error::unavailable("lean", "request failed")
    }
}

fn transport_or(error: &reqwest::Error, decode_reason: &str) -> Error {
    if error.is_timeout() {
        transport(error)
    } else {
        Error::unavailable("lean", decode_reason)
    }
}

#[cfg(test)]
mod test;
