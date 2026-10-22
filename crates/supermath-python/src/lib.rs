//! Python solver tools for the supermath agent.
//!
//! Most of the work before a proof is computation: search small cases, solve
//! a SAT or integer program, compute a Gröbner basis, test a conjecture to
//! ten thousand digits. This crate holds the three `py_*` tools that run that
//! computation in the project's pinned, sandboxed Python service, and the
//! client underneath them. A tool never takes source: it runs a *file* in the
//! project workspace and reports what came back with enough provenance to
//! re-run it.
//!
//! # Entry points
//!
//! - [`Python`] builds the client, the run log, and the tools from a
//!   [`PythonConfig`] and a [`Workspace`](supermath_files::Workspace):
//!   [`PyRun`] (`py_run`), [`PySearchCounterexample`]
//!   (`py_search_counterexample`), and [`PySolvers`] (`py_solvers`).
//! - [`PythonClient`] speaks the service's HTTP API; [`RunRequest`],
//!   [`RunOutcome`], [`SearchOutcome`], and [`Limit`] are its types.
//! - [`Provenance`] is what every result carries; [`canonical_json`] is the
//!   form its `args_sha256` is taken over.
//! - [`PythonObserver`] is the run log, a
//!   [`FileObserver`](supermath_files::FileObserver) for `file_list`.
//! - [`Frame`] is a search frame: a range, a product, or a generator.
//!
//! A Python result is computation, not proof. A run this process executed can
//! back a claim as `checked` on the frame it covered ("holds for all
//! n <= 10^6"), never as `formalised`; a witness is confirmed in Lean before
//! it refutes anything.
//!
//! # Example
//!
//! Build the toolkit; without `PYTHON_URL` every tool answers
//! `BACKEND_UNAVAILABLE` instead of failing to start.
//!
//! ```
//! use std::sync::Arc;
//! use serde_json::json;
//! use supermath_files::{ProjectPath, Workspace, WriteMeta};
//! use supermath_python::{Python, PythonConfig};
//! use supermath_tool::{CallContext, ErrorCode, Project};
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let dir = tempfile::tempdir().unwrap();
//! let workspace = Arc::new(Workspace::new(dir.path()));
//! let path = ProjectPath::parse("python/work/double.py").unwrap();
//! let meta = WriteMeta::new("file_write", "call-1");
//! workspace
//!     .write("demo", &path, "def double(n):\n    return 2 * n\n", None, &meta)
//!     .unwrap();
//!
//! let python = Python::new(&PythonConfig::default(), workspace).unwrap();
//! let py_run = &python.tools()[0];
//! let ctx = CallContext::new(Project::new("demo", "ws:demo").unwrap(), "call-2");
//! let answer = py_run
//!     .call(&ctx, json!({"path": "python/work/double.py", "entry": "double", "args": {"n": 21}}))
//!     .await;
//! assert_eq!(answer.code(), Some(ErrorCode::BackendUnavailable));
//! # });
//! ```
//!
//! # What this crate does not hold
//!
//! It does not run Python itself: the interpreter, the solvers, and the
//! sandbox live in the `python` service (`docker/python/`), and this crate
//! only calls it over HTTP. It does not write source (that is `file_write`),
//! record claims (that is the ledger), or decide a witness refutes anything
//! (that is Lean). It depends on `supermath-tool` and `supermath-files` among
//! the workspace crates, and on no `OpenHuman` crate.

mod client;
mod config;
mod error;
mod frame;
mod observer;
mod provenance;
mod tools;
mod witness;

pub use client::{
    Exception, Execution, Health, Limit, Output, PythonClient, RunOutcome, RunReport, RunRequest,
    SearchOutcome, SearchReport, SearchRequest, Solver, SolverList,
};
pub use config::{
    DEFAULT_MAX_FRAME, DEFAULT_MAX_TIMEOUT_S, DEFAULT_TIMEOUT_ENV, DEFAULT_TIMEOUT_S,
    MAX_FRAME_ENV, MAX_TIMEOUT_ENV, PythonConfig, URL_ENV,
};
pub use error::{Error, Result};
pub use frame::{Frame, MAX_DIMENSIONS, is_identifier};
pub use observer::PythonObserver;
pub use provenance::{Provenance, args_sha256, canonical_json};
pub use tools::{PyRun, PySearchCounterexample, PySolvers, Python};
