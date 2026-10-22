//! A 24-hour on-disk cache of Gemini responses.
//!
//! Each entry is one JSON file named for the SHA-256 of its key, holding the
//! time it was stored and the raw response. A read older than the time to
//! live is a miss. Writes are best effort: a cache that cannot be written
//! costs a repeated request, never a failed call.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// How long a response stays fresh: 24 hours.
pub(crate) const TTL_SECS: u64 = 24 * 3600;

#[derive(Serialize, Deserialize)]
struct Entry {
    stored_at: u64,
    response: Value,
}

/// A cached response and how old it is.
#[derive(Debug)]
pub(crate) struct Hit {
    pub(crate) response: Value,
    pub(crate) age_secs: u64,
}

/// The response cache in one directory.
#[derive(Debug, Clone)]
pub(crate) struct ResponseCache {
    dir: PathBuf,
}

impl ResponseCache {
    pub(crate) fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The cache key for a model and prompt.
    pub(crate) fn key(model: &str, prompt: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(model.as_bytes());
        hasher.update([0]);
        hasher.update(prompt.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.json"))
    }

    /// The fresh entry for `key`, if there is one.
    pub(crate) async fn get(&self, key: &str, now_secs: u64) -> Option<Hit> {
        let bytes = tokio::fs::read(self.path(key)).await.ok()?;
        let entry: Entry = serde_json::from_slice(&bytes).ok()?;
        let age_secs = now_secs.saturating_sub(entry.stored_at);
        (age_secs < TTL_SECS).then_some(Hit {
            response: entry.response,
            age_secs,
        })
    }

    /// Stores `response` under `key`; failures are ignored.
    pub(crate) async fn put(&self, key: &str, response: &Value, now_secs: u64) {
        let entry = Entry {
            stored_at: now_secs,
            response: response.clone(),
        };
        let Ok(bytes) = serde_json::to_vec(&entry) else {
            return;
        };
        if tokio::fs::create_dir_all(&self.dir).await.is_ok() {
            let _ = tokio::fs::write(self.path(key), bytes).await;
        }
    }
}

#[cfg(test)]
mod test;
