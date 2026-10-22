//! The trust rules: what status a claim earns from its source.
//!
//! | Source | Earns |
//! | --- | --- |
//! | `supplied`, `web` | `asserted` (a web page is a lead, not a citation) |
//! | `arxiv` | `cited` |
//! | `tool` (`calc_*`, `py_*`, `lean_*`) | `checked`, from a successful call this process logged for this project, on a stated frame |
//! | `tool` (any other tool) | `asserted` |
//! | `lean` | `formalised` when the evidence port says so, otherwise `asserted` |
//!
//! A call made with [`EvidenceOrigin::Supplied`] never earns more than the
//! source alone would without evidence: the ledger runs no check for it.

use supermath_tool::{CallContext, EvidenceOrigin, Outcome};

use super::{Claim, Ledger};
use crate::error::{Error, Result};
use crate::types::{LeanClaim, Source, Status};

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
            return Ok(Earned::new(Status::Asserted, EvidenceOrigin::Supplied)
                .note("this call's evidence is supplied text, which is not evidence"));
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
            return Ok(Earned::new(Status::Asserted, EvidenceOrigin::Supplied)
                .note("this call's evidence is supplied text, which is not evidence"));
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
