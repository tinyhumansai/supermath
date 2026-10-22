//! [`LeanChecker`]: verify a file in a project and get a verdict with
//! provenance.
//!
//! This is the crate's checking API, separate from any tool. `lean_check`,
//! `lean_build`, `lean_blueprint`, and `lean_search_counterexample` call it,
//! and `supermath-mcp` adapts it to the ledger's `LeanEvidence` port: *verify
//! this declaration at this path, and tell me the verdict and what it ran
//! against*. The ledger does not depend on this crate; the adapter is the
//! composition's job.
//!
//! A verdict is about a file *and* what it imports. [`LeanChecker::is_current`]
//! answers whether a stored [`Provenance`] still describes the project on
//! disk, by digest: when the file or any module it imports changes, every
//! verdict given for the old digests is stale.

mod cache;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use supermath_files::{Language, ProjectPath, Workspace};

use crate::client::{CheckRequest, LeanClient};
use crate::declarations::declarations;
use crate::error::{Error, Result};
use crate::verdict::{CheckReport, Provenance, Reduction, Target, UnknownReason, Verdict, reduce};

pub(crate) use cache::VerdictCache;

/// A report, and how old it is when it came from the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    /// The report.
    pub report: CheckReport,
    /// `Some(age)` when the report was reused because nothing changed.
    pub cached_age: Option<Duration>,
}

/// Checks project files against the Lean service.
///
/// Cloning is cheap; clones share one client and one verdict cache.
#[derive(Debug, Clone)]
pub struct LeanChecker {
    workspace: Arc<Workspace>,
    client: Arc<LeanClient>,
    cache: Arc<VerdictCache>,
}

impl LeanChecker {
    pub(crate) fn new(
        workspace: Arc<Workspace>,
        client: Arc<LeanClient>,
        cache: Arc<VerdictCache>,
    ) -> Self {
        Self {
            workspace,
            client,
            cache,
        }
    }

    /// Checks `path` in project `slug`. `targets` names the declarations
    /// whose axioms are reported; `None` means every declaration the file
    /// makes. `timeout_ms` is the service's wall-time limit.
    ///
    /// A timeout is not an error: it is a report with verdict `unknown`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] for a path that is not a Lean file;
    /// [`Error::Files`] when the file cannot be read; [`Error::NotFound`] when
    /// an explicit target is not declared; [`Error::Unavailable`] when no
    /// service is reachable.
    pub async fn check(
        &self,
        slug: &str,
        path: &ProjectPath,
        targets: Option<&[String]>,
        timeout_ms: u64,
    ) -> Result<CheckOutcome> {
        self.check_with(slug, path, targets, false, timeout_ms)
            .await
    }

    /// Verifies one declaration, the call the ledger's evidence port makes.
    ///
    /// # Errors
    ///
    /// As [`LeanChecker::check`].
    pub async fn verify(
        &self,
        slug: &str,
        path: &ProjectPath,
        declaration: &str,
        timeout_ms: u64,
    ) -> Result<CheckReport> {
        let targets = [declaration.to_owned()];
        Ok(self
            .check(slug, path, Some(&targets), timeout_ms)
            .await?
            .report)
    }

    pub(crate) async fn check_with(
        &self,
        slug: &str,
        path: &ProjectPath,
        targets: Option<&[String]>,
        uses: bool,
        timeout_ms: u64,
    ) -> Result<CheckOutcome> {
        if path.language() != Language::Lean {
            return Err(Error::InvalidInput(format!("{path} is not a Lean file")));
        }
        let file = self.workspace.read(slug, path)?;
        let imports = self.workspace.imports_digest(slug, path)?;
        let found = declarations(&file.text);
        let explicit = targets.is_some();
        let names: Vec<String> = match targets {
            Some(given) => given
                .iter()
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            None => found.iter().map(|d| d.name.clone()).collect(),
        };
        let line_of = |name: &str| {
            found
                .iter()
                .find(|d| d.name == name)
                .map_or(1, |d| u32::try_from(d.line).unwrap_or(u32::MAX))
        };
        let wanted: Vec<Target> = names
            .iter()
            .map(|name| Target {
                name: name.clone(),
                line: line_of(name),
            })
            .collect();

        if let Some(entry) = self.cache.get(slug, path.as_str()) {
            let same = entry.report.provenance.file_sha256 == file.sha256
                && entry.report.provenance.imports_sha256 == imports.sha256
                && entry.report.requested == names
                && (!uses || entry.has_uses);
            if same {
                let age = entry.age();
                return Ok(CheckOutcome {
                    report: entry.report,
                    cached_age: Some(age),
                });
            }
        }

        let request = CheckRequest {
            project: slug.to_owned(),
            path: path.to_string(),
            targets: names.clone(),
            uses,
            timeout_ms,
        };
        let mut provenance = Provenance {
            toolchain: String::new(),
            mathlib: String::new(),
            path: path.to_string(),
            file_sha256: file.sha256.clone(),
            imports_sha256: imports.sha256.clone(),
            elapsed_ms: 0,
            axioms: BTreeMap::default(),
            debt: Vec::new(),
        };
        let reply = match self.client.check(&request).await {
            Ok(reply) => reply,
            Err(Error::Timeout(_)) => {
                return Ok(CheckOutcome {
                    report: unknown_report(provenance, UnknownReason::Timeout, names),
                    cached_age: None,
                });
            }
            Err(other) => return Err(other),
        };
        let reduction = reduce(&reply, &wanted);
        if explicit
            && matches!(
                reduction.verdict,
                Verdict::Checked | Verdict::CheckedWithSorry
            )
            && let Some(missing) = reduction.missing.first()
        {
            return Err(Error::NotFound(format!(
                "{missing} is not declared in {path}"
            )));
        }
        provenance.toolchain.clone_from(&reply.toolchain);
        provenance.mathlib.clone_from(&reply.mathlib);
        provenance.elapsed_ms = reply.elapsed_ms;
        let report = finish(provenance, reduction, reply.sorries, reply.uses, names);
        if report.verdict != Verdict::Unknown {
            self.cache.put(slug, path.as_str(), report.clone(), uses);
        }
        Ok(CheckOutcome {
            report,
            cached_age: None,
        })
    }

    /// The `sorry`s of the last check of `path`, when it was of a file with
    /// the digest `file_sha256`; `None` when there is no such record.
    pub(crate) fn known_sorries(
        &self,
        slug: &str,
        path: &ProjectPath,
        file_sha256: &str,
    ) -> Option<Vec<crate::client::Sorry>> {
        let entry = self.cache.get(slug, path.as_str())?;
        (entry.report.provenance.file_sha256 == file_sha256).then_some(entry.report.sorries)
    }

    /// Whether `provenance` still describes the project on disk: the file and
    /// every project module it imports have the digests the verdict was for.
    ///
    /// # Errors
    ///
    /// [`Error::Files`] when the path in `provenance` is invalid or the file
    /// cannot be read (a deleted file is [`Error::Files`] with `NotFound`).
    pub fn is_current(&self, slug: &str, provenance: &Provenance) -> Result<bool> {
        let path = ProjectPath::parse(&provenance.path)?;
        let file = self.workspace.sha256(slug, &path)?;
        let imports = self.workspace.imports_digest(slug, &path)?;
        Ok(file == provenance.file_sha256 && imports.sha256 == provenance.imports_sha256)
    }
}

fn finish(
    mut provenance: Provenance,
    reduction: Reduction,
    sorries: Vec<crate::client::Sorry>,
    uses: BTreeMap<String, Vec<String>>,
    names: Vec<String>,
) -> CheckReport {
    provenance.axioms = reduction.axioms.clone();
    provenance.debt = reduction.debt;
    let targets = names
        .iter()
        .filter(|n| !reduction.missing.contains(n))
        .cloned()
        .collect();
    CheckReport {
        verdict: reduction.verdict,
        unknown: reduction.unknown,
        provenance,
        errors: reduction.errors,
        error_count: reduction.error_count,
        sorries,
        targets,
        requested: names,
        uses,
    }
}

fn unknown_report(
    provenance: Provenance,
    reason: UnknownReason,
    names: Vec<String>,
) -> CheckReport {
    CheckReport {
        verdict: Verdict::Unknown,
        unknown: Some(reason),
        provenance,
        errors: Vec::new(),
        error_count: 0,
        sorries: Vec::new(),
        targets: names.clone(),
        requested: names,
        uses: BTreeMap::default(),
    }
}
