//! `lean_check`: a Lean file in the project to a verdict with provenance.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Coverage, Envelope, ErrorCode, Exposure, Kind, Permission, Tool,
};

use super::Shared;
use super::wire::{admit_bytes, guard, parse_args, parse_lean_path, valid_name};
use crate::checker::CheckOutcome;
use crate::error::{Error, Result};
use crate::verdict::{CheckReport, Verdict};

/// Checks a file in the project's Lean package as it stands on disk.
#[derive(Debug)]
pub struct LeanCheck {
    shared: Arc<Shared>,
}

impl LeanCheck {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    #[serde(default)]
    targets: Option<Vec<String>>,
}

/// The `data` payload for a report: verdict, provenance, errors, sorries.
pub(super) fn report_data(report: &CheckReport) -> Value {
    json!({
        "verdict": report.verdict,
        "provenance": report.provenance,
        "targets": report.targets,
        "errors": report.errors,
        "error_count": report.error_count,
        "sorries": report.sorries,
    })
}

/// The envelope for a finished check. An `unknown` verdict is not a negative
/// answer: a timeout is `TIMEOUT`, any other cut-off an `unknown` outcome,
/// and both keep the verdict in `data`.
pub(super) fn report_envelope(outcome: &CheckOutcome) -> Envelope {
    let report = &outcome.report;
    let data = report_data(report);
    if report.verdict == Verdict::Unknown {
        let reason = report
            .unknown
            .map_or("the check did not finish", |r| r.describe());
        return match report.unknown {
            Some(crate::verdict::UnknownReason::Timeout) => {
                Envelope::error(ErrorCode::Timeout, reason).with_data(data)
            }
            _ => Envelope::unknown(reason).with_data(data),
        };
    }
    let provenance = &report.provenance;
    let mut coverage = Coverage::new(
        [format!(
            "{} on {}, mathlib {}",
            provenance.path, provenance.toolchain, provenance.mathlib
        )],
        Vec::<String>::new(),
    );
    coverage.cached_age = outcome.cached_age.map(|age| age.as_secs());
    let mut next = Vec::new();
    if let Some(first) = report.sorries.first() {
        next.push(format!(
            "lean_goals {}",
            json!({"path": provenance.path, "line": first.line, "column": first.column})
        ));
        next.push(format!(
            "lean_try_tactics {}",
            json!({"path": provenance.path, "sorry": {"line": first.line, "column": first.column}})
        ));
    }
    Envelope::ok(data).with_coverage(coverage).with_next(next)
}

impl LeanCheck {
    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let path = parse_lean_path(&args.path)?;
        if let Some(targets) = &args.targets
            && (targets.is_empty() || !targets.iter().all(|t| valid_name(t.trim())))
        {
            return Err(Error::InvalidInput(
                "targets must be a non-empty list of declaration names".into(),
            ));
        }
        let slug = ctx.project.slug();
        let file = self.shared.workspace.read(slug, &path)?;
        admit_bytes(ctx, file.text.len())?;
        let outcome = self
            .shared
            .checker
            .check(
                slug,
                &path,
                args.targets.as_deref(),
                self.shared.config.check_timeout_ms,
            )
            .await?;
        Ok(report_envelope(&outcome))
    }
}

#[async_trait]
impl Tool for LeanCheck {
    fn name(&self) -> &'static str {
        "lean_check"
    }

    fn description(&self) -> &'static str {
        "Given a Lean file in the project, checks it with the pinned Lean and Mathlib and returns \
         a verdict: checked, checked_with_sorry (the sorries listed with their goals), failed \
         (errors listed), or unknown (timeout; says nothing about the mathematics). The verdict \
         carries provenance: toolchain, Mathlib revision, file and imports digests, and the axioms \
         of each target declaration; native_decide is named debt. Write the file first with \
         file_write; this never takes source."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "A Lean module, e.g. lean/Supermath/Sendov/Main.lean"},
                "targets": {"type": "array", "items": {"type": "string"}, "minItems": 1,
                            "description": "Declarations whose axioms are reported; default every declaration in the file"}
            },
            "required": ["path"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Direct
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::Timeout,
            ErrorCode::BackendUnavailable,
            ErrorCode::BudgetExceeded,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        guard(ctx, self.run(ctx, args)).await
    }
}
