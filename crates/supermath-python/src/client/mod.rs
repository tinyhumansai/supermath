//! [`PythonClient`]: the HTTP client for the Python run service.
//!
//! It speaks exactly the API in `docs/specs/toolkit/python/README.md`:
//! `GET /health`, `GET /solvers`, `POST /run`, and `POST /search`. Without a
//! configured `PYTHON_URL` every method answers [`Error::Unavailable`], which
//! the tools report as `BACKEND_UNAVAILABLE`.

mod types;

use std::time::Duration;

use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::PythonConfig;
use crate::error::{Error, Result};

pub use types::{
    Exception, Execution, Health, Limit, Output, RunOutcome, RunReport, RunRequest, SearchOutcome,
    SearchReport, SearchRequest, Solver, SolverList,
};

/// Seconds added to a run's own timeout before the HTTP request gives up, so
/// the service's answer to a timeout arrives before the client stops waiting.
const SLACK_S: u64 = 20;
/// How long `health` and `solvers` may take.
const QUICK_S: u64 = 15;

/// The client for the Python service.
///
/// Cheap to clone; clones share one connection pool.
#[derive(Debug, Clone)]
pub struct PythonClient {
    http: reqwest::Client,
    base: Option<String>,
}

impl PythonClient {
    /// A client for `config.url`; with no URL it answers
    /// [`Error::Unavailable`] to every call.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] if the HTTP client cannot be built.
    pub fn new(config: &PythonConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| Error::Unavailable(format!("cannot build the http client: {e}")))?;
        Ok(Self {
            http,
            base: config.url.clone(),
        })
    }

    /// Whether a service URL is configured.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.base.is_some()
    }

    /// `GET /health`: the Python version and the lock digest.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] if the service is unconfigured or unreachable;
    /// [`Error::Upstream`] or [`Error::Decode`] for an unexpected answer.
    pub async fn health(&self) -> Result<Health> {
        self.get("/health").await
    }

    /// `GET /solvers`: the pinned environment's solvers.
    ///
    /// # Errors
    ///
    /// As [`PythonClient::health`].
    pub async fn solvers(&self) -> Result<SolverList> {
        self.get("/solvers").await
    }

    /// `POST /run`: calls one function of a project file.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] and [`Error::NotFound`] when the service
    /// refuses the request (a missing entry point, a missing file);
    /// otherwise as [`PythonClient::health`]. A run that raised or hit a limit
    /// is a successful answer, reported in [`RunReport::outcome`].
    pub async fn run(&self, request: &RunRequest) -> Result<RunReport> {
        let answer: types::Answer = self.post("/run", request, request.timeout_s).await?;
        answer.into_run()
    }

    /// `POST /search`: looks for an input where a predicate is false.
    ///
    /// # Errors
    ///
    /// As [`PythonClient::run`].
    pub async fn search(&self, request: &SearchRequest) -> Result<SearchReport> {
        let answer: types::Answer = self.post("/search", request, request.timeout_s).await?;
        answer.into_search()
    }

    fn url(&self, route: &str) -> Result<String> {
        self.base
            .as_ref()
            .map(|base| format!("{base}{route}"))
            .ok_or_else(|| Error::Unavailable("PYTHON_URL is not set".into()))
    }

    async fn get<T: DeserializeOwned>(&self, route: &str) -> Result<T> {
        let request = self
            .http
            .get(self.url(route)?)
            .timeout(Duration::from_secs(QUICK_S));
        self.send(request).await
    }

    async fn post<B: Serialize + Sync, T: DeserializeOwned>(
        &self,
        route: &str,
        body: &B,
        timeout_s: u64,
    ) -> Result<T> {
        let request = self
            .http
            .post(self.url(route)?)
            .timeout(Duration::from_secs(timeout_s.saturating_add(SLACK_S)))
            .json(body);
        self.send(request).await
    }

    async fn send<T: DeserializeOwned>(&self, request: reqwest::RequestBuilder) -> Result<T> {
        let response = request.send().await.map_err(transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(transport)?;
        if status.is_success() {
            return serde_json::from_slice(&bytes).map_err(|e| Error::Decode(e.to_string()));
        }
        let message = serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
            .unwrap_or_else(|| status.to_string());
        Err(match status {
            StatusCode::BAD_REQUEST => Error::InvalidInput(message),
            StatusCode::NOT_FOUND => Error::NotFound(message),
            other => Error::Upstream {
                status: other.as_u16(),
                message,
            },
        })
    }
}

fn transport(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Unavailable("the service did not answer in time".into())
    } else {
        Error::Unavailable(error.without_url().to_string())
    }
}

#[cfg(test)]
mod test;
