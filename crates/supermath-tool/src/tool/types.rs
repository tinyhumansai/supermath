//! Enums that describe a tool.

use serde::{Deserialize, Serialize};

/// Whether a tool belongs in a model's catalogue from the start, or only
/// once it is looked for. The MCP server publishes it in each tool's
/// `_meta`; every consumer still lists every tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exposure {
    /// For the catalogue from the start.
    Direct,
    /// For a model to find when it needs it.
    Deferred,
}

/// Whether a tool changes state. Declared honestly: the MCP server maps it
/// to `readOnlyHint` and `hive-tools` to a tool's `side_effects`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Reads only.
    Read,
    /// Writes files, the ledger, or other state.
    Write,
}

/// The model backend an agentic tool runs its turn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Backend {
    /// Gemini with Google Search grounding.
    #[serde(rename = "gemini")]
    Gemini,
    /// A full `OpenHuman` agent turn.
    #[serde(rename = "openhuman")]
    OpenHuman,
}

/// Whether a tool is deterministic or delegates to a model turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Deterministic given backend state.
    Primitive,
    /// Runs a model turn on the named backend.
    Agentic(Backend),
}

impl Kind {
    /// Whether this is an [`Kind::Agentic`] tool.
    #[must_use]
    pub fn is_agentic(self) -> bool {
        matches!(self, Kind::Agentic(_))
    }
}
