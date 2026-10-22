//! The trust rules: what status a claim earns from its source.
//!
//! | Source | Earns |
//! | --- | --- |
//! | `supplied`, `web` | `asserted` (a web page is a lead, not a citation) |
//! | `arxiv` (`id`, `version`, and one of `label` or `section`) | `cited`, unless the call's evidence is supplied |
//! | `tool` (`calc_*`, `py_*`, `lean_*`) | `checked`, from a successful call this process logged for this project, on a stated frame |
//! | `tool` (any other tool) | `asserted` |
//! | `lean` | `formalised` when the evidence port says so, otherwise `asserted` |
//!
//! A call made with [`EvidenceOrigin::Supplied`] earns `asserted` whatever
//! the source: the ledger runs no check for it, and supplied text is not
//! evidence, even when it names a paper.

use supermath_tool::{CallContext, EvidenceOrigin, Outcome};

use super::{Claim, Ledger};
use crate::error::{Error, Result};
use crate::types::{Entry, LeanClaim, Source, Status};

/// Tool name prefixes whose successful runs can support `checked`.
const CHECKING_PREFIXES: [&str; 3] = ["calc_", "py_", "lean_"];

/// What a source earned.
#[derive(Debug)]
pub(super) struct Earned {
    pub(super) status: Status,
    pub(super) origin: EvidenceOrigin,
    pub(super) lean: Option<LeanClaim>,
    pub(super) notes: Vec<String>,
}

impl Earned {
    fn new(status: Status, origin: EvidenceOrigin) -> Self {
        Self {
            status,
            origin,
            lean: None,
            notes: Vec::new(),
        }
    }

    fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Keeps the status and origin `old` already holds, for a supplied call
    /// that would otherwise write a lower revision over it.
    pub(super) fn keep(self, old: &Entry) -> Self {
        Self {
            status: old.status,
            origin: old.evidence_origin,
            lean: old.lean.clone(),
            notes: self.notes,
        }
        .note(format!(
            "the claim's {} status was kept; supplied evidence cannot lower it",
            old.status.as_str()
        ))
    }

    /// What any source earns when the call's evidence is supplied.
    fn supplied() -> Self {
        Self::new(Status::Asserted, EvidenceOrigin::Supplied)
            .note("this call's evidence is supplied text, which is not evidence")
    }
}

/// Checks the shape a source must have before it can earn anything: an
/// `arxiv` source names its `id` and `version` and exactly one locator,
/// `label` or `section`. A blank field counts as missing.
pub(super) fn check_source(source: &Source) -> Result<()> {
    let Source::Arxiv {
        id,
        version,
        label,
        section,
    } = source
    else {
        return Ok(());
    };
    let given = |field: Option<&String>| field.is_some_and(|f| !f.trim().is_empty());
    let locators = usize::from(given(label.as_ref())) + usize::from(given(section.as_ref()));
    if !given(Some(id)) || !given(Some(version)) || locators != 1 {
        return Err(Error::InvalidArguments(
            "an arxiv source needs `id`, `version`, and exactly one of `label` or `section`".into(),
        ));
    }
    Ok(())
}

impl Ledger {
    /// Computes the status `claim` earns in `ctx`, running the evidence
    /// ports where the source calls for them.
    pub(super) async fn earn(&self, ctx: &CallContext, claim: &Claim) -> Result<Earned> {
        let origin = ctx.evidence_origin;
        match &claim.source {
            Source::Supplied(_) => Ok(Earned::new(Status::Asserted, EvidenceOrigin::Supplied)),
            Source::Web { .. } => Ok(Earned::new(Status::Asserted, origin)
                .note("a web source is a lead and caps the claim at asserted")),
            Source::Arxiv { .. } if origin == EvidenceOrigin::Supplied => Ok(Earned::supplied()),
            Source::Arxiv { .. } => Ok(Earned::new(Status::Cited, origin)),
            Source::Tool { tool, call_id, .. } => self.earn_from_tool(ctx, claim, tool, call_id),
            Source::Lean {
                path,
                declaration,
                file_sha256,
            } => {
                self.earn_from_lean(ctx, path, declaration, file_sha256)
                    .await
            }
        }
    }

    fn earn_from_tool(
        &self,
        ctx: &CallContext,
        claim: &Claim,
        tool: &str,
        call_id: &str,
    ) -> Result<Earned> {
        if ctx.evidence_origin == EvidenceOrigin::Supplied {
            return Ok(Earned::supplied());
        }
        let Some(record) = self.calls.get(call_id) else {
            return Err(Error::UnrecordedCall(call_id.to_string()));
        };
        let reason = if record.project != ctx.project.slug() {
            Some("it ran in another project")
        } else if record.tool != tool {
            Some("it was a call to a different tool")
        } else if record.outcome != Outcome::Ok {
            Some("it did not end `ok`")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(Error::CallNotUsable {
                call_id: call_id.to_string(),
                reason,
            });
        }
        if !CHECKING_PREFIXES.iter().any(|p| tool.starts_with(p)) {
            return Ok(Earned::new(Status::Asserted, EvidenceOrigin::Collected)
                .note(format!("`{tool}` results cannot support checked")));
        }
        if claim.frame.as_deref().is_none_or(|f| f.trim().is_empty()) {
            return Err(Error::MissingFrame);
        }
        Ok(Earned::new(Status::Checked, EvidenceOrigin::Collected)
            .note("checked holds on the stated frame only"))
    }

    async fn earn_from_lean(
        &self,
        ctx: &CallContext,
        path: &str,
        declaration: &str,
        file_sha256: &str,
    ) -> Result<Earned> {
        if ctx.evidence_origin == EvidenceOrigin::Supplied {
            return Ok(Earned::supplied());
        }
        let verdict = self.evidence.verify(path, declaration).await?;
        if verdict.file_sha256 != file_sha256 {
            return Err(Error::StaleDigest {
                path: path.to_string(),
                given: file_sha256.to_string(),
                current: verdict.file_sha256,
            });
        }
        let status = if verdict.earns_formalised(declaration) {
            Status::Formalised
        } else {
            Status::Asserted
        };
        let mut earned = Earned::new(status, EvidenceOrigin::Collected);
        if status != Status::Formalised {
            earned = earned.note(format!(
                "lean said {:?} with {} sorry and {} debt; formalised needs a clean check",
                verdict.verdict,
                verdict.sorries,
                verdict.debt.len()
            ));
        }
        earned.lean = Some(LeanClaim {
            path: path.to_string(),
            declaration: declaration.to_string(),
            file_sha256: file_sha256.to_string(),
            verdict: verdict.verdict,
            debt: verdict.debt,
            provenance: verdict.provenance,
        });
        Ok(earned)
    }
}
