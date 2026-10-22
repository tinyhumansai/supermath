//! The verdict, its provenance, and the report a check produces.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::client::Sorry;

/// What Lean said about a file, reduced to one of four answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// No errors, no `sorry`, and every axiom of every target is allowed.
    Checked,
    /// No errors, but at least one `sorry` (or `admit`, or `sorryAx`).
    CheckedWithSorry,
    /// Lean reported an error, or a target depends on an untrusted axiom.
    Failed,
    /// The check timed out, ran out of memory, or was cut off. Says nothing
    /// about the mathematics.
    Unknown,
}

impl Verdict {
    /// The verdict's wire name: `checked`, `checked_with_sorry`, `failed`, or
    /// `unknown`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Checked => "checked",
            Self::CheckedWithSorry => "checked_with_sorry",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

/// Why a verdict is [`Verdict::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownReason {
    /// The check ran out of wall time.
    Timeout,
    /// The check ran out of memory.
    Memory,
    /// The check's output passed the size cap.
    OutputLimit,
    /// The service reported a status this client does not understand.
    Other,
}

impl UnknownReason {
    /// A short lowercase phrase for `unknowns`.
    #[must_use]
    pub fn describe(self) -> &'static str {
        match self {
            Self::Timeout => "the check ran out of time",
            Self::Memory => "the check ran out of memory",
            Self::OutputLimit => "the check's output passed the size cap",
            Self::Other => "the service gave an answer this client does not understand",
        }
    }
}

/// One thing wrong with a file: a Lean error, or a refused axiom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// One-based line; the declaration's line for an axiom.
    pub line: u32,
    /// Zero-based column.
    pub column: u32,
    /// Lean's message, trimmed to 400 characters.
    pub message: String,
    /// A stable code when this crate, not Lean, found the problem:
    /// `UNTRUSTED_AXIOM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// What a verdict was about, and what it ran against. This is what the
/// ledger stores with a `formalised` claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The Lean toolchain; empty when the service never answered.
    pub toolchain: String,
    /// The Mathlib commit; empty when the service never answered.
    pub mathlib: String,
    /// The file, relative to the project root.
    pub path: String,
    /// The SHA-256 of the file's bytes.
    pub file_sha256: String,
    /// The SHA-256 over every project module the file imports.
    pub imports_sha256: String,
    /// Wall time the service spent, in milliseconds.
    pub elapsed_ms: u64,
    /// Each target to the axioms it depends on.
    pub axioms: BTreeMap<String, Vec<String>>,
    /// The targets that depend on `Lean.ofReduceBool` or `Lean.trustCompiler`
    /// (`native_decide`): named debt the ledger refuses to call formalised.
    pub debt: Vec<String>,
}

/// The result of checking one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    /// The verdict.
    pub verdict: Verdict,
    /// Why, when the verdict is [`Verdict::Unknown`].
    pub unknown: Option<UnknownReason>,
    /// What the verdict was about.
    pub provenance: Provenance,
    /// At most 20 errors.
    pub errors: Vec<Diagnostic>,
    /// How many errors there were before the cap.
    pub error_count: usize,
    /// Every `sorry`, with its goal.
    pub sorries: Vec<Sorry>,
    /// The targets the verdict speaks for, in order.
    pub targets: Vec<String>,
    /// The target names the check was asked for; the cache key.
    pub(crate) requested: Vec<String>,
    /// Target to the project declarations it uses; empty unless requested.
    pub uses: BTreeMap<String, Vec<String>>,
}
