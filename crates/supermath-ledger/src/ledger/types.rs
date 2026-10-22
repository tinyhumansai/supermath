//! Inputs and outputs of the [`Ledger`](super::Ledger) operations.

use serde_json::Number;

use super::Effective;
use crate::types::{BoundSpec, Direction, Entry, Kind, Source};

/// A claim to record. It carries no status and no evidence origin: both are
/// computed.
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    /// What the entry holds.
    pub kind: Kind,
    /// The finding, in words.
    pub statement: String,
    /// Where it came from.
    pub source: Source,
    /// Claims it depends on.
    pub uses: Vec<String>,
    /// For a `lead`: why it is worth trying.
    pub rationale: Option<String>,
    /// For a `dead_end`: why it failed.
    pub reason: Option<String>,
    /// For a `refutation`: the claim it refutes.
    pub refutes: Option<String>,
    /// For a `tool` source: the finite range the result covered.
    pub frame: Option<String>,
    /// For a bound: its quantity, direction, and value.
    pub bound: Option<BoundSpec>,
}

impl Claim {
    /// A claim of `kind` stating `statement` from `source`, with nothing
    /// else set.
    #[must_use]
    pub fn new(kind: Kind, statement: impl Into<String>, source: Source) -> Self {
        Self {
            kind,
            statement: statement.into(),
            source,
            uses: Vec::new(),
            rationale: None,
            reason: None,
            refutes: None,
            frame: None,
            bound: None,
        }
    }
}

/// What recording did.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    /// The entry that stands for the claim now.
    pub entry: Entry,
    /// Whether a new revision was written; `false` for a repeat.
    pub created: bool,
    /// Why the status is what it is, when the trust rules held it down.
    pub notes: Vec<String>,
}

/// What one read of the ledger's window found.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Recalled {
    /// One entry per claim, at its highest revision, in recall order.
    pub(crate) entries: Vec<Entry>,
    /// Whether memory returned as many notes as the window holds, counted
    /// before notes that are not entries are skipped and revisions merged.
    /// A full window may have left older entries out.
    pub(crate) window_full: bool,
}

/// A bound to offer to the ledger.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundRequest {
    /// The named quantity.
    pub quantity: String,
    /// Which way the bound points.
    pub direction: Direction,
    /// The bound's value.
    pub value: Number,
    /// Where the bound came from.
    pub source: Source,
    /// For a `tool` source: the finite range the result covered.
    pub frame: Option<String>,
}

/// The result of offering a bound.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundOutcome {
    /// Whether the offered bound became the best one.
    pub improved: bool,
    /// The best bound so far, or the offered one if nothing better exists.
    pub best: Entry,
    /// The best bound's effective status, the one `ledger_claims` would
    /// report: no stronger than what it uses or its Lean file allows now.
    pub best_effective: Effective,
    /// Whether the read that found the best bound filled the window, so an
    /// older, better bound may have been missed.
    pub window_full: bool,
    /// What recording the offer did: a `bound` if it improved, otherwise a
    /// `fact`.
    pub recorded: Recorded,
}
