//! The evidence ports: what the ledger needs from the rest of the toolkit to
//! promote a claim, without depending on the crates that produce it.
//!
//! [`LeanEvidence`] runs Lean on one declaration and reports the verdict;
//! `supermath-mcp` adapts the real `lean_check` to it. A `tool` source is
//! checked against `supermath-tool`'s [`CallLog`](supermath_tool::CallLog)
//! instead, so it needs no port of its own.
//!
//! [`LeanVerdict::earns_formalised`] is the single place that decides whether
//! a verdict is enough: a clean check, no `sorry`, no named debt, and every
//! axiom of the declaration inside [`AXIOM_ALLOWLIST`].

#[cfg(any(test, feature = "test-support"))]
mod fake;

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeLeanEvidence;

/// The axioms a `formalised` claim may rest on.
pub const AXIOM_ALLOWLIST: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];

/// What `lean_check` said about a file, reduced to four verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeanVerdictKind {
    /// No errors, no `sorry`, every axiom in the allowlist.
    Checked,
    /// No errors, but at least one `sorry`.
    CheckedWithSorry,
    /// Lean reported an error.
    Failed,
    /// The check timed out or was refused.
    Unknown,
}

/// The result of running Lean on one declaration in one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeanVerdict {
    /// The verdict.
    pub verdict: LeanVerdictKind,
    /// The digest of the file as Lean checked it.
    pub file_sha256: String,
    /// How many `sorry`s the declaration depends on.
    pub sorries: u32,
    /// Declarations depending on `native_decide` or the compiler; named debt
    /// that keeps a claim from being `formalised`.
    pub debt: Vec<String>,
    /// The axioms of each target declaration.
    pub axioms: BTreeMap<String, Vec<String>>,
    /// The toolchain, Mathlib revision, and timing, as `lean_check` reports.
    pub provenance: Value,
}

impl LeanVerdict {
    /// Whether this verdict earns `formalised` for `declaration`.
    ///
    /// All of these must hold: the verdict is [`LeanVerdictKind::Checked`];
    /// no `sorry` remains; no debt is named; and `declaration` is among the
    /// verdict's targets with every axiom in [`AXIOM_ALLOWLIST`]. A verdict
    /// that does not mention the declaration proves nothing about it.
    #[must_use]
    pub fn earns_formalised(&self, declaration: &str) -> bool {
        self.verdict == LeanVerdictKind::Checked
            && self.sorries == 0
            && self.debt.is_empty()
            && self.axioms.get(declaration).is_some_and(|axioms| {
                axioms
                    .iter()
                    .all(|axiom| AXIOM_ALLOWLIST.contains(&axiom.as_str()))
            })
    }
}

/// Runs Lean on a file in this process and reports the verdict for one
/// declaration.
#[async_trait]
pub trait LeanEvidence: Send + Sync {
    /// Checks `path` as it stands on disk and reports the verdict for
    /// `declaration`, with the digest of the file that was checked.
    ///
    /// # Errors
    ///
    /// [`Error::EvidenceNotFound`](crate::Error::EvidenceNotFound) for a
    /// missing file or declaration,
    /// [`Error::EvidenceUnavailable`](crate::Error::EvidenceUnavailable) when
    /// Lean is down, and [`Error::EvidenceTimeout`](crate::Error::EvidenceTimeout)
    /// when the check ran out of time.
    async fn verify(&self, path: &str, declaration: &str) -> Result<LeanVerdict>;
}

#[cfg(test)]
mod test;
