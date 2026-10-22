//! The call log: a port every host can implement, and an in-memory
//! implementation.
//!
//! The log records one [`CallRecord`] per call, by `call_id`. It holds no
//! arguments and no payloads, only what is needed to find a call again and
//! see how it ended.

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::{Envelope, ErrorCode, Outcome};

/// One recorded call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallRecord {
    /// The host-assigned id of the call.
    pub call_id: String,
    /// The tool that was called.
    pub tool: String,
    /// The project slug the call acted on.
    pub project: String,
    /// How the call ended.
    pub outcome: Outcome,
    /// The error code, when the outcome is `error`.
    pub code: Option<ErrorCode>,
    /// Wall time in milliseconds.
    pub elapsed_ms: u64,
}

impl CallRecord {
    /// A record of `tool` having answered `envelope`.
    #[must_use]
    pub fn new(
        call_id: impl Into<String>,
        tool: impl Into<String>,
        project: impl Into<String>,
        envelope: &Envelope,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool: tool.into(),
            project: project.into(),
            outcome: envelope.outcome,
            code: envelope.code(),
            elapsed_ms,
        }
    }
}

/// Where calls are recorded. A host may back it with a file or a database;
/// [`InMemoryCallLog`] keeps them for this process.
pub trait CallLog: Send + Sync {
    /// Appends a record.
    fn record(&self, record: CallRecord);
    /// The record with this id, if the call was logged.
    fn get(&self, call_id: &str) -> Option<CallRecord>;
    /// Every record, oldest first.
    fn all(&self) -> Vec<CallRecord>;
}

/// A [`CallLog`] held in memory, for the life of the process.
#[derive(Debug, Default)]
pub struct InMemoryCallLog {
    records: Mutex<Vec<CallRecord>>,
}

impl InMemoryCallLog {
    /// An empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<CallRecord>> {
        // A panic while holding the lock cannot leave a Vec half-pushed.
        self.records.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CallLog for InMemoryCallLog {
    fn record(&self, record: CallRecord) {
        self.lock().push(record);
    }

    fn get(&self, call_id: &str) -> Option<CallRecord> {
        self.lock()
            .iter()
            .rev()
            .find(|r| r.call_id == call_id)
            .cloned()
    }

    fn all(&self) -> Vec<CallRecord> {
        self.lock().clone()
    }
}

#[cfg(test)]
mod test;
