//! Reducing what Lean said to a [`Verdict`].
//!
//! [`reduce`] is a pure function of the service's reply, so every row of the
//! verdict table is pinned by a unit test without a service:
//!
//! | Verdict | When |
//! | --- | --- |
//! | `checked` | no errors, no `sorry`, every axiom allowed |
//! | `checked_with_sorry` | no errors, but a `sorry`, `admit`, or `sorryAx` |
//! | `failed` | an error, or an axiom outside the allowlist |
//! | `unknown` | timeout, out of memory, or output cut off |
//!
//! The allowlist is `propext`, `Classical.choice`, and `Quot.sound`.
//! `Lean.ofReduceBool` and `Lean.trustCompiler` (from `native_decide`) are
//! named debt: the verdict stays `checked` and the declaration is listed in
//! [`Provenance::debt`].

mod types;

use std::collections::BTreeMap;

use crate::client::{CheckReply, Status};

pub use types::{CheckReport, Diagnostic, Provenance, UnknownReason, Verdict};

/// The axioms a `checked` verdict may rest on.
pub const ALLOWED_AXIOMS: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];
/// The axioms `native_decide` leans on: allowed, but recorded as debt.
pub const DEBT_AXIOMS: [&str; 2] = ["Lean.ofReduceBool", "Lean.trustCompiler"];
/// The axiom every `sorry` elaborates to.
pub const SORRY_AXIOM: &str = "sorryAx";
/// The code on a [`Diagnostic`] for an axiom outside the allowlist.
pub const UNTRUSTED_AXIOM: &str = "UNTRUSTED_AXIOM";

/// Most errors a report lists.
pub const MAX_ERRORS: usize = 20;
/// Longest error message a report keeps, in characters.
pub const MAX_MESSAGE_CHARS: usize = 400;

const SORRY_WARNING: &str = "declaration uses 'sorry'";

/// A declaration whose axioms are reported, and the line it sits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) name: String,
    pub(crate) line: u32,
}

/// What [`reduce`] concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reduction {
    pub(crate) verdict: Verdict,
    pub(crate) unknown: Option<UnknownReason>,
    pub(crate) errors: Vec<Diagnostic>,
    pub(crate) error_count: usize,
    pub(crate) axioms: BTreeMap<String, Vec<String>>,
    pub(crate) debt: Vec<String>,
    /// Targets the environment does not have.
    pub(crate) missing: Vec<String>,
}

/// Reduces a service reply to a verdict for `targets`.
pub(crate) fn reduce(reply: &CheckReply, targets: &[Target]) -> Reduction {
    let mut reduction = Reduction {
        verdict: Verdict::Unknown,
        unknown: None,
        errors: Vec::new(),
        error_count: 0,
        axioms: BTreeMap::new(),
        debt: Vec::new(),
        missing: Vec::new(),
    };
    let cut_off = match reply.status {
        Status::Ok => None,
        Status::Timeout => Some(UnknownReason::Timeout),
        Status::Memory => Some(UnknownReason::Memory),
        Status::OutputLimit => Some(UnknownReason::OutputLimit),
        _ => Some(UnknownReason::Other),
    };
    if cut_off.is_some() {
        reduction.unknown = cut_off;
        return reduction;
    }

    let mut errors: Vec<Diagnostic> = reply
        .messages
        .iter()
        .filter(|m| m.severity == "error")
        .map(|m| Diagnostic {
            line: m.line,
            column: m.column,
            message: trim_message(&m.text),
            code: None,
        })
        .collect();
    let mut sorry = !reply.sorries.is_empty()
        || reply
            .messages
            .iter()
            .any(|m| m.text.contains(SORRY_WARNING));

    for target in targets {
        match reply.axioms.get(&target.name) {
            Some(Some(list)) => {
                reduction.axioms.insert(target.name.clone(), list.clone());
            }
            _ => reduction.missing.push(target.name.clone()),
        }
    }
    for (name, list) in &reduction.axioms {
        sorry |= list.iter().any(|a| a == SORRY_AXIOM);
        if list.iter().any(|a| DEBT_AXIOMS.contains(&a.as_str())) {
            reduction.debt.push(name.clone());
        }
        let untrusted: Vec<&str> = list
            .iter()
            .map(String::as_str)
            .filter(|a| {
                !ALLOWED_AXIOMS.contains(a) && !DEBT_AXIOMS.contains(a) && *a != SORRY_AXIOM
            })
            .collect();
        if !untrusted.is_empty() {
            let line = targets
                .iter()
                .find(|t| &t.name == name)
                .map_or(1, |t| t.line);
            errors.push(Diagnostic {
                line,
                column: 0,
                message: trim_message(&format!(
                    "{name} depends on untrusted axioms: {}",
                    untrusted.join(", ")
                )),
                code: Some(UNTRUSTED_AXIOM.to_owned()),
            });
        }
    }

    reduction.error_count = errors.len();
    errors.truncate(MAX_ERRORS);
    reduction.errors = errors;
    reduction.verdict = if reduction.error_count > 0 {
        Verdict::Failed
    } else if sorry {
        Verdict::CheckedWithSorry
    } else {
        Verdict::Checked
    };
    reduction
}

/// Trims a message to [`MAX_MESSAGE_CHARS`] characters, marking the cut.
pub(crate) fn trim_message(text: &str) -> String {
    match text.char_indices().nth(MAX_MESSAGE_CHARS) {
        Some((cut, _)) => format!("{}…", text[..cut].trim_end()),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod test;
