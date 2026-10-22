//! The in-memory verdict cache, keyed by project and path.
//!
//! An entry holds the last report for a file. It is valid for a new check
//! only while the file's digest, its imports' digest, and the target list are
//! the same; the observer reads it to tell `file_list` the last verdict and
//! the digest it was for. Unknown verdicts are never cached: they say nothing
//! worth reusing. The cache lives as long as the process; a restart simply
//! re-checks.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::verdict::CheckReport;

/// One cached report and when it was made.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) report: CheckReport,
    pub(crate) has_uses: bool,
    pub(crate) at: Instant,
}

impl Entry {
    pub(crate) fn age(&self) -> Duration {
        self.at.elapsed()
    }
}

/// Reports by `(project slug, path)`.
#[derive(Debug, Default)]
pub(crate) struct VerdictCache {
    entries: Mutex<HashMap<(String, String), Entry>>,
}

impl VerdictCache {
    pub(crate) fn get(&self, slug: &str, path: &str) -> Option<Entry> {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&(slug.to_owned(), path.to_owned()))
            .cloned()
    }

    pub(crate) fn put(&self, slug: &str, path: &str, report: CheckReport, has_uses: bool) {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                (slug.to_owned(), path.to_owned()),
                Entry {
                    report,
                    has_uses,
                    at: Instant::now(),
                },
            );
    }
}
