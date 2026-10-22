//! The project workspace: Lean and Python files on disk, and the four
//! `file_*` tools that are the only way source reaches them.
//!
//! Every tool that runs code runs a *file in this workspace*, never a string
//! of source passed as an argument. A project is a directory
//! `<SUPERMATH_PROJECTS>/<slug>/` holding a Lean package (`lean/Supermath/…`)
//! and a Python package (`python/work/…`). This crate owns the layout, the
//! [path rules](ProjectPath) and [import rules](imports), atomic writes
//! guarded by `if_sha256`, and the [write log](WriteLog).
//!
//! # Entry points
//!
//! - [`Workspace`] resolves, reads, digests, lists, and writes files. The
//!   Lean and Python crates use it to find a file by [`ProjectPath`], read
//!   it, and compute [`Workspace::sha256`] and [`Workspace::imports_digest`].
//! - [`WriteLog`] (from [`Workspace::write_log`]) reads what a call wrote.
//! - [`FileObserver`] is the port the Lean and Python crates implement so
//!   [`FileList`] can show each file's last result.
//! - [`FileWrite`], [`FileEdit`], [`FileRead`], [`FileList`] are the tools;
//!   [`file_tools`] builds all four.
//! - [`FilesConfig`] reads `SUPERMATH_PROJECTS`.
//!
//! # Example
//!
//! ```
//! use supermath_files::{ProjectPath, Workspace, WriteMeta};
//!
//! let dir = tempfile::tempdir().unwrap();
//! let workspace = Workspace::new(dir.path());
//! let path = ProjectPath::parse("python/work/hello.py").unwrap();
//!
//! let meta = WriteMeta::new("file_write", "call-1");
//! let written = workspace.write("demo", &path, "import math\n", None, &meta).unwrap();
//! assert_eq!(workspace.sha256("demo", &path).unwrap(), written.sha256);
//!
//! // Writing again without the digest just seen is a conflict.
//! assert!(workspace.write("demo", &path, "import math\n", None, &meta).is_err());
//! ```
//!
//! # What this crate does not hold
//!
//! It does not check Lean or run Python: those belong to `supermath-lean` and
//! `supermath-python`, which depend on this crate, not the reverse. It does
//! not know a project's slug except through the argument a host passes (the
//! slug comes from the call context, never from a tool's arguments), and it
//! has no network access.

mod config;
mod error;
pub mod imports;
mod observer;
pub mod outline;
mod path;
mod tools;
mod workspace;
mod write_log;

pub use config::{DEFAULT_PROJECTS, FilesConfig, PROJECTS_ENV};
pub use error::{Error, Result};
pub use observer::{FileObserver, LastResult};
pub use path::{Language, MAX_PATH_BYTES, ProjectPath};
pub use tools::{FileEdit, FileList, FileRead, FileWrite, MAX_READ_LINES, file_tools};
pub use workspace::{
    ImportedModule, ImportsDigest, MAX_FILE_BYTES, SourceFile, Workspace, WriteMeta, WriteOutcome,
    sha256_hex,
};
pub use write_log::{WriteLog, WriteRecord};
