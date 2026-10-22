//! The on-disk response cache under `SUPERMATH_CACHE`.
//!
//! Responses are keyed by the SHA-256 of the request URL. Each file holds the
//! time it was stored (eight big-endian bytes of milliseconds) and then the
//! body. A search is trusted for 24 hours; a versioned paper never changes, so
//! it is trusted forever. With no directory configured the cache is off and
//! every request goes to the network. Writes are best effort: a cache that
//! cannot be written costs speed, never correctness.

use std::path::PathBuf;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::Clock;

/// How long a search, or the metadata of an unversioned id, is trusted.
pub const SEARCH_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// A cached body and how old it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedBody {
    /// The stored response body.
    pub body: Vec<u8>,
    /// Milliseconds since it was stored.
    pub age_ms: u64,
}

/// A disk cache of upstream responses, keyed by URL.
#[derive(Debug, Clone)]
pub struct ResponseCache {
    dir: Option<PathBuf>,
    clock: Arc<dyn Clock>,
}

impl ResponseCache {
    /// A cache under `dir`, or a disabled one for `None`.
    #[must_use]
    pub fn new(dir: Option<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        Self { dir, clock }
    }

    fn path(&self, url: &str) -> Option<PathBuf> {
        let dir = self.dir.as_ref()?;
        Some(dir.join(hex::encode(Sha256::digest(url.as_bytes()))))
    }

    /// The stored body for `url`, if one exists and is younger than
    /// `max_age_ms` (`None` accepts any age).
    pub async fn get(&self, url: &str, max_age_ms: Option<u64>) -> Option<CachedBody> {
        let raw = tokio::fs::read(self.path(url)?).await.ok()?;
        let (stamp, body) = raw.split_at_checked(8)?;
        let stored = u64::from_be_bytes(stamp.try_into().ok()?);
        let age_ms = self.clock.now_ms().saturating_sub(stored);
        if max_age_ms.is_some_and(|max| age_ms > max) {
            return None;
        }
        Some(CachedBody {
            body: body.to_vec(),
            age_ms,
        })
    }

    /// Stores `body` for `url`. Failures are ignored.
    pub async fn put(&self, url: &str, body: &[u8]) {
        let Some(path) = self.path(url) else {
            return;
        };
        let Some(dir) = path.parent() else {
            return;
        };
        if tokio::fs::create_dir_all(dir).await.is_err() {
            return;
        }
        let mut raw = self.clock.now_ms().to_be_bytes().to_vec();
        raw.extend_from_slice(body);
        let partial = path.with_extension("partial");
        if tokio::fs::write(&partial, raw).await.is_ok()
            && tokio::fs::rename(&partial, &path).await.is_err()
        {
            let _ = tokio::fs::remove_file(&partial).await;
        }
    }
}

#[cfg(test)]
mod test;
