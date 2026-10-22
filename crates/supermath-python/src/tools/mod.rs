//! The three Python tools and what they share.
//!
//! [`Python`] builds the client, the run log, and the tools from one
//! [`PythonConfig`] and one project [`Workspace`]. The tools never take
//! source: they run a file already written with `file_write`, so every result
//! can name the file digest it was about.

mod run;
mod search;
mod solvers;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;
use supermath_files::{Language, ProjectPath, Workspace};
use supermath_tool::{Budget, Demand, Tool};

pub use run::PyRun;
pub use search::PySearchCounterexample;
pub use solvers::PySolvers;

use crate::client::Execution;
use crate::error::{Error, Result};
use crate::{Provenance, PythonClient, PythonConfig, PythonObserver};

/// The 64 KB cap on a run's JSON arguments.
pub(crate) const MAX_ARGS_BYTES: usize = 64 * 1024;

/// What every tool needs: the service, the workspace, the run log, the caps.
#[derive(Debug)]
pub(crate) struct Shared {
    pub(crate) client: PythonClient,
    pub(crate) workspace: Arc<Workspace>,
    pub(crate) observer: Arc<PythonObserver>,
    pub(crate) config: PythonConfig,
}

/// A project file read and digested before it runs.
#[derive(Debug)]
pub(crate) struct Prepared {
    pub(crate) path: ProjectPath,
    pub(crate) file_sha256: String,
    pub(crate) imports_sha256: String,
}

impl Shared {
    /// The timeout to run with, in seconds.
    ///
    /// A spent compute budget (no wall time left) is refused whatever was
    /// asked, so nothing reaches the service. An explicit `timeout_s` over the
    /// configured maximum, or over the call's wall-time budget, is refused. A
    /// call that names none gets the default, shortened to the budget when
    /// the budget is tighter.
    pub(crate) fn timeout(&self, budget: &Budget, requested: Option<u64>) -> Result<u64> {
        if budget.wall_time_ms == 0 {
            return Err(Error::BudgetExceeded(
                "the call's compute budget is spent: no wall time is left to run in".into(),
            ));
        }
        let Some(seconds) = requested else {
            let budget_s = budget.wall_time_ms.div_ceil(1000).max(1);
            return Ok(self.config.default_timeout_s.min(budget_s));
        };
        if seconds == 0 {
            return Err(Error::InvalidInput("timeout_s must be at least 1".into()));
        }
        if seconds > self.config.max_timeout_s {
            return Err(Error::BudgetExceeded(format!(
                "timeout_s {seconds} is over the cap of {}",
                self.config.max_timeout_s
            )));
        }
        budget
            .admit(&Demand {
                wall_time_ms: seconds.saturating_mul(1000),
                ..Demand::default()
            })
            .map_err(|refusal| Error::BudgetExceeded(refusal.to_string()))?;
        Ok(seconds)
    }

    /// Reads `raw` from the project and digests it and its imports.
    pub(crate) fn prepare(&self, slug: &str, raw: &str) -> Result<Prepared> {
        let path = ProjectPath::parse(raw)?;
        if path.language() != Language::Python {
            return Err(Error::InvalidInput(format!(
                "{raw} is not a Python file; py tools run python/work/*.py"
            )));
        }
        let file = self.workspace.read(slug, &path)?;
        let imports = self.workspace.imports_digest(slug, &path)?;
        Ok(Prepared {
            path,
            file_sha256: file.sha256,
            imports_sha256: imports.sha256,
        })
    }

    /// Joins what was read with what the service reported, refusing a run of a
    /// different version of the file than the one this process digested.
    pub(crate) fn provenance(
        prepared: &Prepared,
        entry: &str,
        args_sha256: String,
        seed: u64,
        execution: &Execution,
    ) -> Result<Provenance> {
        if execution.file_sha256 != prepared.file_sha256 {
            return Err(Error::Unavailable(format!(
                "the service ran a different version of {} than the workspace holds; \
                 check that it mounts the same projects directory",
                prepared.path
            )));
        }
        Ok(Provenance {
            path: prepared.path.to_string(),
            file_sha256: prepared.file_sha256.clone(),
            imports_sha256: prepared.imports_sha256.clone(),
            entry: entry.to_owned(),
            args_sha256,
            seed,
            python: execution.python.clone(),
            lock_sha256: execution.lock_sha256.clone(),
            elapsed_ms: execution.elapsed_ms,
            peak_rss_mb: execution.peak_rss_mb,
        })
    }
}

/// Parses a tool's JSON arguments into its typed form.
pub(crate) fn parse_args<T: DeserializeOwned>(args: Value) -> Result<T> {
    serde_json::from_value(args).map_err(|e| Error::InvalidInput(e.to_string()))
}

/// Whether `value` holds a non-integer JSON number anywhere.
pub(crate) fn contains_float(value: &Value) -> bool {
    match value {
        Value::Number(n) => n.is_f64(),
        Value::Array(items) => items.iter().any(contains_float),
        Value::Object(map) => map.values().any(contains_float),
        _ => false,
    }
}

/// The Python toolkit: client, run log, and the three tools.
///
/// ```
/// use std::sync::Arc;
/// use supermath_files::Workspace;
/// use supermath_python::{Python, PythonConfig};
///
/// let python = Python::new(&PythonConfig::default(), Arc::new(Workspace::new("/tmp/projects")))
///     .unwrap();
/// let names: Vec<&str> = python.tools().iter().map(|t| t.name()).collect();
/// assert_eq!(names, ["py_run", "py_search_counterexample", "py_solvers"]);
/// ```
#[derive(Debug, Clone)]
pub struct Python {
    shared: Arc<Shared>,
}

impl Python {
    /// Builds the toolkit over `workspace`, with a fresh run log.
    ///
    /// # Errors
    ///
    /// [`Error::Unavailable`] if the HTTP client cannot be built. A missing
    /// `PYTHON_URL` is not an error here: the tools answer
    /// `BACKEND_UNAVAILABLE` when called.
    pub fn new(config: &PythonConfig, workspace: Arc<Workspace>) -> Result<Self> {
        Ok(Self {
            shared: Arc::new(Shared {
                client: PythonClient::new(config)?,
                workspace,
                observer: Arc::new(PythonObserver::new()),
                config: config.clone(),
            }),
        })
    }

    /// `py_run`, `py_search_counterexample`, and `py_solvers`, in that order.
    #[must_use]
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(PyRun::new(self.shared.clone())),
            Arc::new(PySearchCounterexample::new(self.shared.clone())),
            Arc::new(PySolvers::new(self.shared.clone())),
        ]
    }

    /// The run log, to register with `file_list` as the Python
    /// [`FileObserver`](supermath_files::FileObserver).
    #[must_use]
    pub fn observer(&self) -> Arc<PythonObserver> {
        self.shared.observer.clone()
    }

    /// The service client, for `check` commands that report `GET /health`.
    #[must_use]
    pub fn client(&self) -> PythonClient {
        self.shared.client.clone()
    }
}

#[cfg(test)]
mod test;
