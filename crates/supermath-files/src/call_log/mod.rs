//! The file call log: a [`CallLog`] that outlives the process.
//!
//! Every call is appended to `projects/<slug>/.log/calls.jsonl`, one JSON
//! [`CallRecord`] per line. A host that runs each tool call as its own
//! process (an executable tool, or one MCP server per agent) shares this file
//! between them, so `ledger_record` can check a `tool` source quoting a call
//! some earlier process made.
//!
//! [`CallLog`] is infallible, so failures are absorbed rather than raised: a
//! record that cannot be written is lost, and a line that cannot be read is
//! skipped. Either way the call is simply not found, and the ledger refuses a
//! claim citing it. A lost record never makes a claim stronger.
//!
//! ```
//! use serde_json::json;
//! use supermath_files::FileCallLog;
//! use supermath_tool::{CallLog, CallRecord, Envelope};
//!
//! let dir = tempfile::tempdir().unwrap();
//! let path = dir.path().join(".log/calls.jsonl");
//! FileCallLog::new(&path).record(CallRecord::new(
//!     "call-1", "calc_factor", "demo", &Envelope::ok(json!(1)), 3,
//! ));
//! // Another handle, as another process would open it, sees the call.
//! assert!(FileCallLog::new(&path).get("call-1").is_some());
//! ```

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use supermath_tool::{CallLog, CallRecord};

/// A handle on one project's `calls.jsonl`.
#[derive(Debug, Clone)]
pub struct FileCallLog {
    path: PathBuf,
}

impl FileCallLog {
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

    fn append(&self, record: &CallRecord) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut line = serde_json::to_string(record)?;
        line.push('\n');
        // One `write_all` of one line to an append-mode file: concurrent
        // processes do not interleave within a line.
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?
            .write_all(line.as_bytes())
    }
}

impl CallLog for FileCallLog {
    fn record(&self, record: CallRecord) {
        // A lost record only makes the call unfindable; see the module docs.
        let _ = self.append(&record);
    }

    fn get(&self, call_id: &str) -> Option<CallRecord> {
        self.all().into_iter().rev().find(|r| r.call_id == call_id)
    }

    fn all(&self) -> Vec<CallRecord> {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        text.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

#[cfg(test)]
mod test;
