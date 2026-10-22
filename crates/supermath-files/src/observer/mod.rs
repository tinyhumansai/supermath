//! [`FileObserver`]: the port through which `file_list` learns each file's
//! last result.
//!
//! The files crate does not know the Lean or Python domains. Each domain
//! crate implements this trait, and the composition (`supermath-mcp`)
//! registers the implementations with [`FileList`](crate::FileList).

use serde::{Deserialize, Serialize};

use crate::path::{Language, ProjectPath};

/// The last result a domain recorded for a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastResult {
    /// A short label for the result: a Lean verdict (`ok`, `error`, …) or a
    /// Python run outcome.
    pub result: String,
    /// The digest of the file the result was for. When it differs from the
    /// file's current digest, the result is stale.
    pub sha256: String,
}

/// Reports the last result of one language's files.
pub trait FileObserver: Send + Sync {
    /// The language this observer reports on.
    fn language(&self) -> Language;

    /// The last result recorded for `path` in project `project` (the slug),
    /// and the digest it was for; `None` if the file was never checked or run.
    fn last_result(&self, project: &str, path: &ProjectPath) -> Option<LastResult>;
}
