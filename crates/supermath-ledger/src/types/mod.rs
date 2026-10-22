//! The ledger's entry and the vocabulary around it.
//!
//! An [`Entry`] is one finding: a kind, a statement, a status, a source, and
//! edges to the claims it uses. Its serde form is the wire form stored in
//! `CortexDB` and pinned by a unit test; renaming a field breaks every entry
//! already recorded.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use supermath_tool::EvidenceOrigin;

use crate::evidence::LeanVerdictKind;

/// What an entry holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A definition, theorem, or technique from the literature.
    Fact,
    /// A claim this project established or is establishing.
    Result,
    /// The problem as stated, its variants, and the assumptions read into it.
    Intake,
    /// The best known bound on a named quantity, with its source.
    Bound,
    /// An idea worth trying, with why.
    Lead,
    /// An approach that failed, with the reason.
    DeadEnd,
    /// A counterexample, with the statement it refutes and the witness.
    Refutation,
}

impl Kind {
    /// The wire spelling, as used in the `kind:` label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Result => "result",
            Self::Intake => "intake",
            Self::Bound => "bound",
            Self::Lead => "lead",
            Self::DeadEnd => "dead_end",
            Self::Refutation => "refutation",
        }
    }
}

/// How well a claim is known. Statuses are ordered: each needs more evidence
/// than the one before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Someone said it; the default.
    Asserted,
    /// A source says it, with a locator.
    Cited,
    /// It held on a stated finite range.
    Checked,
    /// The Lean kernel accepted a statement of it, in this run.
    Formalised,
}

impl Status {
    /// The wire spelling, as used in the `status:` label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asserted => "asserted",
            Self::Cited => "cited",
            Self::Checked => "checked",
            Self::Formalised => "formalised",
        }
    }
}

/// Whether a Lean statement says what the informal one says.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    /// No one has compared them; every new Lean claim starts here.
    #[default]
    NotReviewed,
    /// A reviewer found the statements to agree.
    Accepted,
    /// A reviewer found a gap: the claim is a theorem about something else.
    Disputed,
}

/// Which way a bound points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The quantity is at most the value.
    Upper,
    /// The quantity is at least the value.
    Lower,
}

/// Where a claim came from. Exactly one of these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// A paper on arXiv, located by exactly one of `label` or `section`;
    /// the ledger refuses one with neither or both.
    Arxiv {
        /// The arXiv identifier, e.g. `2012.04125`.
        id: String,
        /// The version, e.g. `v3`.
        version: String,
        /// The label of the statement, e.g. `thm:main`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        /// The section, when there is no label.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        section: Option<String>,
    },
    /// A declaration in a file of the project's Lean package.
    Lean {
        /// The file, relative to the project.
        path: String,
        /// The declaration that states the claim.
        declaration: String,
        /// The digest of the file the caller saw.
        file_sha256: String,
    },
    /// A computation a tool in this process ran.
    Tool {
        /// The tool that ran.
        tool: String,
        /// The call's id in the call log.
        call_id: String,
        /// A digest of the call's arguments.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args_digest: Option<String>,
    },
    /// A page a `web_*` tool surfaced.
    Web {
        /// The page's URL.
        url: String,
        /// The page's title.
        title: String,
        /// The page's class, e.g. `forum`.
        class: String,
        /// When the page was retrieved.
        retrieved_at: String,
    },
    /// Free text that arrived as text.
    Supplied(String),
}

/// The Lean file and declaration that state a claim, and the verdict
/// [`LeanEvidence`](crate::LeanEvidence) returned for them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeanClaim {
    /// The file, relative to the project.
    pub path: String,
    /// The declaration that states the claim.
    pub declaration: String,
    /// The digest the verdict holds for.
    pub file_sha256: String,
    /// The verdict of the check.
    pub verdict: LeanVerdictKind,
    /// Declarations that depend on `native_decide` or the compiler.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub debt: Vec<String>,
    /// The toolchain, Mathlib revision, and timing the check recorded.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub provenance: Value,
}

/// The quantity, direction, and value of a `bound` entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundSpec {
    /// The named quantity, whitespace-normalised.
    pub quantity: String,
    /// Which way the bound points.
    pub direction: Direction,
    /// The bound's value.
    pub value: serde_json::Number,
}

/// One finding in the ledger. Never edited: a change writes a new
/// [`Entry::revision`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// `clm_` and a digest of the kind, statement, and source; for a `lean`
    /// source, of the kind and the declaration's path and name only.
    pub id: String,
    /// Which revision of the claim this is, starting at 1. Readers take the
    /// highest.
    pub revision: u32,
    /// What the entry holds.
    pub kind: Kind,
    /// The finding, in words.
    pub statement: String,
    /// How well it is known, as earned when it was recorded.
    pub status: Status,
    /// Whether this process produced the evidence behind `status`.
    pub evidence_origin: EvidenceOrigin,
    /// Where it came from.
    pub source: Source,
    /// The Lean file and verdict, for a claim stated in Lean.
    pub lean: Option<LeanClaim>,
    /// Whether the Lean statement says what the informal one says.
    pub alignment: Alignment,
    /// The claims this one depends on.
    pub uses: Vec<String>,
    /// The event id of the revision this one replaces.
    pub supersedes: Option<String>,
    /// For a `lead`: why it is worth trying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// For a `dead_end`: why it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// For a `refutation`: the claim it refutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refutes: Option<String>,
    /// For a `tool` source: the finite range the result covered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
    /// For a bound: its quantity, direction, and value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<BoundSpec>,
}

impl Entry {
    /// The `kind:`, `status:`, and `claim:` labels this entry is written with.
    #[must_use]
    pub fn labels(&self) -> [String; 3] {
        [
            format!("kind:{}", self.kind.as_str()),
            format!("status:{}", self.status.as_str()),
            format!("claim:{}", self.id),
        ]
    }
}

#[cfg(test)]
mod test;
