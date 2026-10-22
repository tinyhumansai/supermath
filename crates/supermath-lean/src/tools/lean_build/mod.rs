//! `lean_build`: every Lean file of the project, checked in dependency order.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_files::ProjectPath;
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, clamp_limit,
};

use super::Shared;
use super::project::{ProjectFile, lean_files};
use super::wire::{coverage, guard, parse_args, parse_cursor};
use crate::declarations::declarations;
use crate::error::{Error, Result};
use crate::verdict::{SORRY_AXIOM, Verdict};

/// Checks the whole package, reusing verdicts of files that did not change.
#[derive(Debug)]
pub struct LeanBuild {
    shared: Arc<Shared>,
}

impl LeanBuild {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    #[serde(default)]
    limit: Option<u64>,
    #[serde(default)]
    cursor: Option<String>,
}

/// One file's row in the build report.
fn row(path: &ProjectPath, verdict: &str, detail: &Value) -> Value {
    let mut row = json!({"path": path.as_str(), "verdict": verdict});
    if let (Some(map), Some(extra)) = (row.as_object_mut(), detail.as_object()) {
        map.extend(extra.clone());
    }
    row
}

/// The index of `verdict` in the build's four counters.
fn bucket(verdict: Verdict) -> usize {
    match verdict {
        Verdict::Checked => 0,
        Verdict::CheckedWithSorry => 1,
        Verdict::Failed => 2,
        Verdict::Unknown => 3,
    }
}

impl LeanBuild {
    /// Checks one file and builds its row, with the counter it belongs to.
    async fn check_row(
        &self,
        slug: &str,
        file: &ProjectFile,
        timeout_ms: u64,
    ) -> Result<(usize, Value)> {
        let outcome = match self
            .shared
            .checker
            .check(slug, &file.path, None, timeout_ms)
            .await
        {
            Ok(outcome) => outcome,
            Err(e @ Error::Unavailable { .. }) => return Err(e),
            Err(other) => {
                return Ok((
                    3,
                    row(&file.path, "unknown", &json!({"reason": other.to_string()})),
                ));
            }
        };
        let report = outcome.report;
        let own_sorry_decls: Vec<String> = declarations(&file.text)
            .into_iter()
            .filter(|d| d.has_sorry)
            .map(|d| d.name)
            .collect();
        let depends_on_sorry: Vec<&String> = report
            .provenance
            .axioms
            .iter()
            .filter(|(name, axioms)| {
                axioms.iter().any(|a| a == SORRY_AXIOM) && !own_sorry_decls.contains(name)
            })
            .map(|(name, _)| name)
            .collect();
        let detail = json!({
            "file_sha256": report.provenance.file_sha256,
            "error_count": report.error_count,
            "sorry_count": report.sorries.len(),
            "cached": outcome.cached_age.is_some(),
            "depends_on_sorry": depends_on_sorry,
            "debt": report.provenance.debt,
            "reason": report.unknown.map(crate::verdict::UnknownReason::describe),
        });
        Ok((
            bucket(report.verdict),
            row(&file.path, report.verdict.as_str(), &detail),
        ))
    }

    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let offset = parse_cursor(args.cursor.as_deref())?;
        let limit = clamp_limit(args.limit).min(ctx.budget.rows.max(1));
        let slug = ctx.project.slug();
        let found = lean_files(&self.shared.workspace, slug)?;
        let files = found.ordered;
        let budget = Duration::from_millis(self.shared.config.build_timeout_ms);
        let started = Instant::now();

        let mut rows = Vec::with_capacity(files.len());
        let mut counts = [0usize; 4];
        let mut skipped = 0usize;
        for file in &files {
            let remaining = budget.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                skipped += 1;
                counts[3] += 1;
                rows.push(row(
                    &file.path,
                    "unknown",
                    &json!({"reason": "the build ran out of time before this file"}),
                ));
                continue;
            }
            let timeout_ms = self
                .shared
                .config
                .check_timeout_ms
                .min(u64::try_from(remaining.as_millis()).unwrap_or(u64::MAX));
            let (counter, built) = self.check_row(slug, file, timeout_ms).await?;
            counts[counter] += 1;
            rows.push(built);
        }
        for file in &found.unreadable {
            counts[3] += 1;
            rows.push(row(
                &file.path,
                "unknown",
                &json!({"reason": format!("the file could not be read: {}", file.reason)}),
            ));
        }

        let total = rows.len();
        let page: Vec<Value> = rows.into_iter().skip(offset).take(limit).collect();
        let data = json!({
            "files": page,
            "total": total,
            "summary": {
                "checked": counts[0],
                "checked_with_sorry": counts[1],
                "failed": counts[2],
                "unknown": counts[3],
            },
        });
        let mut envelope = if counts[3] > 0 {
            Envelope::unknown(format!(
                "{} file(s) have no verdict; see their reasons",
                counts[3]
            ))
        } else {
            Envelope::ok(Value::Null)
        };
        envelope = envelope.with_data(data).with_coverage(coverage(
            format!("{total} Lean file(s) in dependency order"),
            &[],
        ));
        if !found.unreadable.is_empty() {
            let paths: Vec<&str> = found.unreadable.iter().map(|f| f.path.as_str()).collect();
            envelope = envelope.with_unknown(format!(
                "{} Lean file(s) could not be read and were not checked: {}",
                paths.len(),
                paths.join(", ")
            ));
        }
        if skipped > 0 {
            envelope = envelope.with_unknown(
                "the build budget ran out; call again to continue with cached verdicts",
            );
        }
        let next = offset + limit;
        Ok(if next < total {
            envelope.with_cursor(next.to_string())
        } else {
            envelope
        })
    }
}

#[async_trait]
impl Tool for LeanBuild {
    fn name(&self) -> &'static str {
        "lean_build"
    }

    fn description(&self) -> &'static str {
        "Given the project, checks every file of its Lean package in dependency order and returns \
         a verdict per file as lean_check would, plus the declarations that depend on a sorry in \
         another file. Files whose digests and imports are unchanged reuse their cached verdicts. \
         Budget: 15 minutes for the build. Paged."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "limit": {"type": "integer", "minimum": 1, "maximum": 50},
                "cursor": {"type": "string", "description": "From a previous call; continues the list"}
            },
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[ErrorCode::InvalidInput, ErrorCode::BackendUnavailable]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        guard(ctx, self.run(ctx, args)).await
    }
}
