//! [`LeanBridge`]: the adapter from `supermath-lean`'s checker to the
//! ledger's [`LeanEvidence`] port.
//!
//! The ledger never imports Lean. When `ledger_record` is given a `lean`
//! source it asks its port for a verdict on one declaration; this adapter
//! answers by running the same checker `lean_check` runs, in the project the
//! toolkit was composed for. The ledger decides what the verdict earns, so
//! nothing here sets a claim status. `tests/ports.rs` drives it end to end
//! against a loopback Lean service, because a check report is only built by
//! the checker.
//!
//! Failures keep their meaning across the port:
//!
//! | `supermath-lean` | [`LeanEvidence`] |
//! | --- | --- |
//! | a path that is not a Lean file, a missing file, an undeclared name | `EvidenceNotFound` |
//! | no service, or an unreachable one | `EvidenceUnavailable` |
//! | a check that ran out of wall time (the checker reports it as an `unknown` verdict) | `EvidenceTimeout` |
//! | any other verdict, including out of memory | the verdict, as `unknown` |

use async_trait::async_trait;
use serde_json::Value;
use supermath_files::ProjectPath;
use supermath_lean::{CheckReport, Error as LeanError, LeanChecker, UnknownReason, Verdict};
use supermath_ledger::{Error, LeanEvidence, LeanVerdict, LeanVerdictKind, Result};

/// Runs `lean_check`'s checker for the ledger, in one project.
#[derive(Debug, Clone)]
pub struct LeanBridge {
    checker: LeanChecker,
    slug: String,
    timeout_ms: u64,
}

impl LeanBridge {
    /// A bridge over `checker` for project `slug`, giving each check
    /// `timeout_ms` of wall time.
    #[must_use]
    pub fn new(checker: LeanChecker, slug: impl Into<String>, timeout_ms: u64) -> Self {
        Self {
            checker,
            slug: slug.into(),
            timeout_ms,
        }
    }
}

#[async_trait]
impl LeanEvidence for LeanBridge {
    async fn verify(&self, path: &str, declaration: &str) -> Result<LeanVerdict> {
        let not_found = || Error::EvidenceNotFound {
            path: path.to_owned(),
            declaration: declaration.to_owned(),
        };
        let parsed = ProjectPath::parse(path).map_err(|_| not_found())?;
        let report = self
            .checker
            .verify(&self.slug, &parsed, declaration, self.timeout_ms)
            .await
            .map_err(|error| match error {
                LeanError::InvalidInput(_) | LeanError::NotFound(_) | LeanError::Files(_) => {
                    not_found()
                }
                other => Error::EvidenceUnavailable(other.to_string()),
            })?;
        if report.unknown == Some(UnknownReason::Timeout) {
            return Err(Error::EvidenceTimeout);
        }
        Ok(verdict(&report))
    }
}

/// The ledger's view of a check report.
fn verdict(report: &CheckReport) -> LeanVerdict {
    let kind = match report.verdict {
        Verdict::Checked => LeanVerdictKind::Checked,
        Verdict::CheckedWithSorry => LeanVerdictKind::CheckedWithSorry,
        Verdict::Failed => LeanVerdictKind::Failed,
        Verdict::Unknown => LeanVerdictKind::Unknown,
    };
    LeanVerdict {
        verdict: kind,
        file_sha256: report.provenance.file_sha256.clone(),
        sorries: u32::try_from(report.sorries.len()).unwrap_or(u32::MAX),
        debt: report.provenance.debt.clone(),
        axioms: report.provenance.axioms.clone(),
        provenance: serde_json::to_value(&report.provenance).unwrap_or(Value::Null),
    }
}
