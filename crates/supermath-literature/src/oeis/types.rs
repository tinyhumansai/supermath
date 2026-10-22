//! OEIS rows.

use serde::Serialize;
use serde_json::Value;

/// Terms shown per sequence.
pub(super) const MAX_TERMS: usize = 30;
/// Formula lines shown per sequence.
pub(super) const MAX_FORMULA_LINES: usize = 5;
/// Reference lines shown per sequence.
pub(super) const MAX_REFERENCE_LINES: usize = 5;

/// One OEIS sequence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sequence {
    /// Always `oeis`.
    pub source: &'static str,
    /// The A-number, such as `A000045`.
    pub id: String,
    /// The sequence's name.
    pub name: String,
    /// Up to 30 terms; a term too large for a JSON integer is a string.
    pub terms: Vec<Value>,
    /// Up to five formula lines.
    pub formula: Vec<String>,
    /// Up to five reference lines.
    pub references: Vec<String>,
}

/// A page of OEIS matches.
#[derive(Debug, Clone, PartialEq)]
pub struct OeisPage {
    /// The sequences on this page.
    pub sequences: Vec<Sequence>,
    /// How many sequences matched in all, when OEIS says.
    pub total: Option<u64>,
    /// The offset to ask for next, if more remain.
    pub next: Option<u64>,
    /// How old the cached answer was, in seconds, if it was cached.
    pub cached_age_secs: Option<u64>,
}
