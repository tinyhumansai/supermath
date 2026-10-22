//! The write log's record.

use serde::{Deserialize, Serialize};

/// One line of `projects/<slug>/.log/writes.jsonl`: a successful write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteRecord {
    /// The path written, e.g. `lean/Supermath/Sendov/Main.lean`.
    pub path: String,
    /// The digest before the write; `null` for a new file.
    pub old_sha256: Option<String>,
    /// The digest after the write.
    pub new_sha256: String,
    /// The tool that wrote, e.g. `file_edit`.
    pub tool: String,
    /// The call (or turn) that wrote, so a ledger can say which call produced
    /// a file.
    pub call_id: String,
    /// Wall-clock time of the write, in milliseconds since the Unix epoch.
    pub at_ms: u64,
}
