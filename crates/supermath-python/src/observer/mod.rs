//! [`PythonObserver`]: the run log `file_list` reads.
//!
//! Every `py_run` and `py_search_counterexample` call records the outcome of
//! the file it ran, with the digest the file had. `file_list` shows the last
//! result per file and marks it stale when the digest no longer matches.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use supermath_files::{FileObserver, Language, LastResult, ProjectPath};

/// An in-memory log of the last Python result per `(project, file)`.
///
/// ```
/// use supermath_files::{FileObserver, Language, ProjectPath};
/// use supermath_python::PythonObserver;
///
/// let observer = PythonObserver::new();
/// let path = ProjectPath::parse("python/work/a.py").unwrap();
/// assert!(observer.last_result("demo", &path).is_none());
///
/// observer.record("demo", &path, "returned", "abc");
/// let last = observer.last_result("demo", &path).unwrap();
/// assert_eq!((last.result.as_str(), last.sha256.as_str()), ("returned", "abc"));
/// assert_eq!(observer.language(), Language::Python);
/// ```
#[derive(Debug, Default)]
pub struct PythonObserver {
    results: Mutex<HashMap<(String, String), LastResult>>,
}

impl PythonObserver {
    /// An empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that running `path` in `project` ended in `result` (a short
    /// label such as `returned`, `raised`, or `unknown`) while the file had
    /// digest `sha256`. A later record for the same file replaces it.
    pub fn record(&self, project: &str, path: &ProjectPath, result: &str, sha256: &str) {
        self.lock().insert(
            (project.to_owned(), path.to_string()),
            LastResult {
                result: result.to_owned(),
                sha256: sha256.to_owned(),
            },
        );
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<(String, String), LastResult>> {
        self.results.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl FileObserver for PythonObserver {
    fn language(&self) -> Language {
        Language::Python
    }

    fn last_result(&self, project: &str, path: &ProjectPath) -> Option<LastResult> {
        self.lock()
            .get(&(project.to_owned(), path.to_string()))
            .cloned()
    }
}

#[cfg(test)]
mod test;
