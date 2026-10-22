//! [`LeanObserver`]: the Lean side of `file_list`'s "last result" column.
//!
//! It reports the verdict of the last check of each file, and the digest that
//! verdict was for. `file_list` compares that digest to the file's current
//! one and marks the result stale when they differ. A verdict also goes stale
//! when a project module the file imports changes; the observer reports that
//! by returning a digest no file can have (`imports-changed:<digest>`), so the
//! same comparison catches it.

use std::sync::Arc;

use supermath_files::{FileObserver, Language, LastResult, ProjectPath, Workspace};

use crate::checker::VerdictCache;

/// Reports the last Lean verdict of a file. Build it with
/// [`Lean::observer`](crate::Lean::observer) and register it with
/// `file_list`.
#[derive(Debug, Clone)]
pub struct LeanObserver {
    workspace: Arc<Workspace>,
    cache: Arc<VerdictCache>,
}

impl LeanObserver {
    pub(crate) fn new(workspace: Arc<Workspace>, cache: Arc<VerdictCache>) -> Self {
        Self { workspace, cache }
    }
}

impl FileObserver for LeanObserver {
    fn language(&self) -> Language {
        Language::Lean
    }

    fn last_result(&self, project: &str, path: &ProjectPath) -> Option<LastResult> {
        let entry = self.cache.get(project, path.as_str())?;
        let provenance = &entry.report.provenance;
        let imports_moved = self
            .workspace
            .imports_digest(project, path)
            .is_ok_and(|now| now.sha256 != provenance.imports_sha256);
        let sha256 = if imports_moved {
            format!("imports-changed:{}", provenance.file_sha256)
        } else {
            provenance.file_sha256.clone()
        };
        Some(LastResult {
            result: entry.report.verdict.as_str().to_owned(),
            sha256,
        })
    }
}

#[cfg(test)]
mod test;
