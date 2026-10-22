//! Durable memory in `CortexDB`, scoped per project.
//!
//! One `CortexDB` server holds every project this deployment works on. Each
//! project's notes live at
//!
//! ```text
//! {scope_root}/project:{slug}/app:brain
//! ```
//!
//! and the scope is built here from configuration and the project slug only.
//! No model output and no tool argument reaches a scope, so an agent cannot
//! read or write another project's memory by naming it. The self-hosted
//! server cannot mint per-project tokens, so this file is the boundary;
//! math-superagent's `src/orchestrator/cortex.rs` records the same trade-off.
//!
//! A write waits for the `indexed` barrier and checks the server reached it.
//! A write that stored bytes it has not indexed is an error, because a note
//! that can never be recalled is worse than one that was never stored: the
//! run would report it as kept.

mod pack;
mod project;
#[cfg(any(test, feature = "test-support"))]
pub mod stub;

use std::time::Duration;

use serde_json::{Value, json};

use crate::error::{Error, Result};

pub use pack::{reached_barrier, render_pack};
pub use project::{Project, slugify};

/// The barrier a durable write waits for.
pub const WRITE_BARRIER: &str = "indexed";

/// The scope segment a project's durable notes go to.
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

    /// The scope `project`'s durable notes are filed under.
    #[must_use]
    pub fn scope(&self, project: &Project) -> String {
        format!(
            "{}/project:{}/{BRAIN_STORE}",
            self.scope_root,
            project.slug()
        )
    }

    /// Recalls what memory holds about `query` for `project`, rendered as
    /// Markdown.
    ///
    /// Returns `None` when nothing is recorded, including when the server
    /// answers `404` for a scope nothing has been written to yet.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MemoryTransport`] when the server cannot be reached
    /// in time, and [`Error::MemoryRefused`] when it refuses the request.
    pub async fn recall(&self, project: &Project, query: &str) -> Result<Option<String>> {
        let Some(pack) = self
            .recall_pack(project, query, &RECALL_LAYERS, RECALL_LIMIT)
            .await?
        else {
            return Ok(None);
        };
        Ok(render_pack(&pack, &RECALL_LAYERS))
    }

    /// Recalls the text of the notes recorded in `project`'s scope that are
    /// closest to `query`, at most `limit` of them.
    ///
    /// This is the read the ledger builds on: it takes the events layer only,
    /// unrendered, so each note's own JSON can be parsed and filtered by the
    /// caller. A scope nothing has been written to yields no notes.
    ///
    /// # Errors
    ///
    /// The same as [`Memory::recall`].
    pub async fn recall_notes(
        &self,
        project: &Project,
        query: &str,
        limit: u64,
    ) -> Result<Vec<String>> {
        let pack = self.recall_pack(project, query, &["events"], limit).await?;
        Ok(pack.as_ref().map(pack::event_texts).unwrap_or_default())
    }

    async fn recall_pack(
        &self,
        project: &Project,
        query: &str,
        layers: &[&str],
        limit: u64,
    ) -> Result<Option<Value>> {
        let limits: serde_json::Map<String, Value> = layers
            .iter()
            .map(|layer| (pack::singular(layer).to_string(), json!(limit)))
            .collect();
        let body = json!({
            "scope": self.scope(project),
            // One scope, read directly. math-superagent found `descend`
            // silently missing a populated scope, so no traversal is used.
            "view": "granular",
            "query": query,
            "include": layers,
            "budgets": { "per_layer_limits": limits },
            "citation_mode": "none",
        });
        let response = self
            .send("recall", "/v1/recall", &body, RECALL_TIMEOUT)
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Self::read_json("recall", response).await.map(Some)
    }

    /// Stores `note` in `project`'s durable memory and waits until it is
    /// indexed. Returns the event id the server assigned.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MemoryTransport`] or [`Error::MemoryRefused`] as
    /// [`Memory::recall`] does, and [`Error::MemoryNotIndexed`] when the
    /// server stored the note without indexing it.
    pub async fn remember(&self, project: &Project, note: &str, labels: &[&str]) -> Result<String> {
        let body = json!({
            "scope": self.scope(project),
            "modality": "observation",
            "content": { "kind": "text", "text": note },
            "context": { "labels": labels },
            // The server bounds keys at 64 characters and answers a reused key
            // with a different body by `409`, so the key is a content digest.
            "idempotency_key": idempotency_key(&self.scope(project), note),
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

    /// Checks that the server is ready to store recallable memories.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MemoryTransport`] when the server cannot be reached,
    /// and [`Error::MemoryNotReady`] when it reports itself not ready, or
    /// degraded (for example, on mock embeddings that recall nothing).
    pub async fn ready(&self) -> Result<()> {
        let request = self
            .client
            .get(format!("{}/v1/admin/ready", self.base_url))
            .bearer_auth(&self.api_key)
            .send();
        let response = tokio::time::timeout(RECALL_TIMEOUT, request)
            .await
            .map_err(|_| Error::MemoryTransport {
                operation: "ready",
                detail: format!("no answer within {} seconds", RECALL_TIMEOUT.as_secs()),
            })?
            .map_err(|error| Error::MemoryTransport {
                operation: "ready",
                detail: error.to_string(),
            })?;
        // A `503` carries the same report as a `200`; the body is the answer.
        let report: Value = response
            .json()
            .await
            .map_err(|error| Error::MemoryTransport {
                operation: "ready",
                detail: format!("invalid JSON: {error}"),
            })?;
        pack::readiness(&report).map_err(Error::MemoryNotReady)
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

/// A 16-hex-digit FNV-1a digest of `parts`, each followed by a NUL byte.
///
/// The ledger derives claim ids from it. Not cryptographic: it only has to
/// make the same finding the same id.
#[must_use]
pub fn content_digest(parts: &[&str]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in parts.iter().flat_map(|part| part.bytes().chain([0])) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
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
