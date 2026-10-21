//! Durable memory in `CortexDB`, scoped per problem.
//!
//! One `CortexDB` server holds every problem this deployment works on. Each
//! problem's notes live at
//!
//! ```text
//! {scope_root}/project:{slug}/app:brain
//! ```
//!
//! and the scope is built here from configuration and the problem slug only.
//! No model output and no tool argument reaches a scope, so an agent cannot
//! read or write another problem's memory by naming it. The self-hosted
//! server cannot mint per-problem tokens, so this file is the boundary;
//! math-superagent's `src/orchestrator/cortex.rs` records the same trade-off.
//!
//! A write waits for the `indexed` barrier and checks the server reached it.
//! A write that stored bytes it has not indexed is an error, because a note
//! that can never be recalled is worse than one that was never stored: the
//! run would report it as kept.

mod pack;

use std::time::Duration;

use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::problem::Problem;

pub use pack::{reached_barrier, render_pack};

/// The barrier a durable write waits for.
pub const WRITE_BARRIER: &str = "indexed";

/// The scope segment a problem's durable notes go to.
///
/// An `app:` segment because `CortexDB` v0.10 accepts only the scope types in
/// its deployment's `allowed_scope_types`, and refuses math-superagent's
/// `store:` with `UNREGISTERED_SCOPE_TYPE`.
const BRAIN_STORE: &str = "app:brain";

/// How many entries each recalled layer may return.
const RECALL_LIMIT: u64 = 8;

/// The layers a recall asks for, most direct first.
const RECALL_LAYERS: [&str; 3] = ["events", "facts", "beliefs"];

/// How long a recall may take before the run goes on without it.
const RECALL_TIMEOUT: Duration = Duration::from_secs(30);

/// How long a write may take. Indexing calls an extraction model, so a
/// write is much slower than a recall.
const WRITE_TIMEOUT: Duration = Duration::from_secs(120);

/// A `CortexDB` client bound to one deployment's scope root.
#[derive(Debug, Clone)]
pub struct Memory {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    scope_root: String,
}

impl Memory {
    /// Builds a client for the server at `base_url`.
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        scope_root: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            scope_root: scope_root.into(),
        }
    }

    /// The scope `problem`'s durable notes are filed under.
    #[must_use]
    pub fn scope(&self, problem: &Problem) -> String {
        format!(
            "{}/project:{}/{BRAIN_STORE}",
            self.scope_root,
            problem.slug()
        )
    }

    /// Recalls what memory holds about `query` for `problem`, rendered as
    /// Markdown.
    ///
    /// Returns `None` when nothing is recorded, including when the server
    /// answers `404` for a scope nothing has been written to yet.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MemoryTransport`] when the server cannot be reached
    /// in time, and [`Error::MemoryRefused`] when it refuses the request.
    pub async fn recall(&self, problem: &Problem, query: &str) -> Result<Option<String>> {
        let limits: serde_json::Map<String, Value> = RECALL_LAYERS
            .iter()
            .map(|layer| (pack::singular(layer).to_string(), json!(RECALL_LIMIT)))
            .collect();
        let body = json!({
            "scope": self.scope(problem),
            // One scope, read directly. math-superagent found `descend`
            // silently missing a populated scope, so no traversal is used.
            "view": "granular",
            "query": query,
            "include": RECALL_LAYERS,
            "budgets": { "per_layer_limits": limits },
            "citation_mode": "none",
        });
        let response = self
            .send("recall", "/v1/recall", &body, RECALL_TIMEOUT)
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let pack = Self::read_json("recall", response).await?;
        Ok(render_pack(&pack, &RECALL_LAYERS))
    }

    /// Stores `note` in `problem`'s durable memory and waits until it is
    /// indexed. Returns the event id the server assigned.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MemoryTransport`] or [`Error::MemoryRefused`] as
    /// [`Memory::recall`] does, and [`Error::MemoryNotIndexed`] when the
    /// server stored the note without indexing it.
    pub async fn remember(&self, problem: &Problem, note: &str, labels: &[&str]) -> Result<String> {
        let body = json!({
            "scope": self.scope(problem),
            "modality": "observation",
            "content": { "kind": "text", "text": note },
            "context": { "labels": labels },
            // The server bounds keys at 64 characters and answers a reused key
            // with a different body by `409`, so the key is a content digest.
            "idempotency_key": idempotency_key(&self.scope(problem), note),
        });
        let path = format!("/v1/experience?wait={WRITE_BARRIER}");
        let response = self.send("remember", &path, &body, WRITE_TIMEOUT).await?;
        let answer = Self::read_json("remember", response).await?;
        reached_barrier(&answer, WRITE_BARRIER)?;
        Ok(answer
            .get("event_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string())
    }

    async fn send(
        &self,
        operation: &'static str,
        path: &str,
        body: &Value,
        timeout: Duration,
    ) -> Result<reqwest::Response> {
        let request = self
            .client
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(&self.api_key)
            .json(body)
            .send();
        tokio::time::timeout(timeout, request)
            .await
            .map_err(|_| Error::MemoryTransport {
                operation,
                detail: format!("no answer within {} seconds", timeout.as_secs()),
            })?
            .map_err(|error| Error::MemoryTransport {
                operation,
                detail: error.to_string(),
            })
    }

    async fn read_json(operation: &'static str, response: reqwest::Response) -> Result<Value> {
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| Error::MemoryTransport {
                operation,
                detail: error.to_string(),
            })?;
        if !status.is_success() {
            return Err(Error::MemoryRefused {
                operation,
                status: status.as_u16(),
                detail: pack::refusal_detail(&text),
            });
        }
        serde_json::from_str(&text).map_err(|error| Error::MemoryTransport {
            operation,
            detail: format!("invalid JSON: {error}"),
        })
    }
}

/// A stable 64-character key for one note in one scope.
///
/// FNV-1a over the scope and the note, rendered as four 16-digit hex words
/// with different offsets. Not cryptographic; it only has to make a genuine
/// repeat the same key and a genuine rewrite a different one.
fn idempotency_key(scope: &str, note: &str) -> String {
    use std::fmt::Write as _;
    (0u64..4).fold(String::with_capacity(64), |mut key, salt| {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ salt;
        for byte in scope.bytes().chain([0]).chain(note.bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        // Writing to a `String` cannot fail.
        let _ = write!(key, "{hash:016x}");
        key
    })
}

#[cfg(test)]
mod test;
