//! The write log: an append-only record of every successful write.
//!
//! Every write is appended to `projects/<slug>/.log/writes.jsonl`, one JSON
//! [`WriteRecord`] per line. Agentic tools read this log, not their child's
//! reply, to learn what a turn wrote. A refused write leaves no line.
//!
//! ```
//! use supermath_files::{WriteLog, WriteRecord};
//!
//! let dir = tempfile::tempdir().unwrap();
//! let log = WriteLog::new(dir.path().join(".log/writes.jsonl"));
//! assert!(log.entries().unwrap().is_empty());
//! log.append(&WriteRecord {
//!     path: "python/work/a.py".into(),
//!     old_sha256: None,
//!     new_sha256: "ab".into(),
//!     tool: "file_write".into(),
//!     call_id: "call-1".into(),
//!     at_ms: 0,
//! })
//! .unwrap();
//! assert_eq!(log.for_call("call-1").unwrap().len(), 1);
//! ```

mod types;

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub use types::WriteRecord;

/// A handle on one project's `writes.jsonl`.
#[derive(Debug, Clone)]
pub struct WriteLog {
    path: PathBuf,
}

impl WriteLog {
    /// A handle on the log at `path`. The file need not exist yet.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Where the log lives.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one record as a single line, creating the log and its
    /// directory on first use.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the log cannot be created or written.
    pub fn append(&self, record: &WriteRecord) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::io("create the write log directory", e))?;
        }
        let mut line = serde_json::to_string(record)
            .map_err(|e| Error::io("encode a write record", e.into()))?;
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| Error::io("open the write log", e))?;
        file.write_all(line.as_bytes())
            .map_err(|e| Error::io("append to the write log", e))
    }

    /// Every record, oldest first. A missing log is empty.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the log cannot be read; [`Error::LogCorrupt`] for a
    /// line that is not a record.
    pub fn entries(&self) -> Result<Vec<WriteRecord>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(Error::io("read the write log", e)),
        };
        text.lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(index, line)| {
                serde_json::from_str(line).map_err(|source| Error::LogCorrupt {
                    line: index + 1,
                    source,
                })
            })
            .collect()
    }

    /// The records written by `call_id`, oldest first: what a turn wrote.
    ///
    /// # Errors
    ///
    /// As [`WriteLog::entries`].
    pub fn for_call(&self, call_id: &str) -> Result<Vec<WriteRecord>> {
        Ok(self
            .entries()?
            .into_iter()
            .filter(|r| r.call_id == call_id)
            .collect())
    }
}

#[cfg(test)]
mod test;
