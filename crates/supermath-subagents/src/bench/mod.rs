//! The formalization benchmark: thirty informal statements, run through
//! `agent_formalize` per model, reporting the `checked` rate.
//!
//! The statements live in `bench/formalize/statements.json`, checked into the
//! crate. [`run_formalize`] drives any `agent_formalize` tool over them and
//! counts a case `checked` when the tool's own `lean_check` verdict is
//! `checked` or `checked_with_sorry`; the live test in
//! `crates/supermath-agent/tests/live.rs` composes the real toolkit and calls it
//! once per model.

use serde::{Deserialize, Serialize};
use serde_json::json;
use supermath_tool::{CallContext, ErrorCode, Outcome, Tool, Usage};

use crate::error::{Error, Result};

/// The checked-in benchmark set.
const STATEMENTS: &str = include_str!("../../bench/formalize/statements.json");

/// One informal statement and the Mathlib theorem it formalizes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizeCase {
    /// A unique slug, such as `even-add`.
    pub id: String,
    /// The area of mathematics.
    pub area: String,
    /// The informal statement given to the tool.
    pub statement: String,
    /// The Mathlib name of the theorem it formalizes.
    pub expected: String,
}

impl FormalizeCase {
    /// The Lean file the benchmark asks the tool to write for this case.
    #[must_use]
    pub fn path(&self) -> String {
        let name: String = self
            .id
            .split('-')
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                    .unwrap_or_default()
            })
            .collect();
        format!("lean/Supermath/Bench/{name}.lean")
    }
}

/// The thirty cases.
///
/// # Errors
///
/// [`Error::Bench`] if the checked-in file does not parse; a test pins that it
/// does.
pub fn formalize_cases() -> Result<Vec<FormalizeCase>> {
    serde_json::from_str(STATEMENTS).map_err(|error| Error::Bench(error.to_string()))
}

/// What one case produced.
#[derive(Debug, Clone)]
pub struct CaseResult {
    /// The case's id.
    pub id: String,
    /// The tool's outcome.
    pub outcome: Outcome,
    /// The target file's verdict, as the tool computed it.
    pub verdict: Option<String>,
    /// The error code, if the tool refused.
    pub error: Option<ErrorCode>,
    /// What the turn spent.
    pub usage: Option<Usage>,
}

impl CaseResult {
    /// Whether the statement compiled: `checked` or `checked_with_sorry`.
    #[must_use]
    pub fn checked(&self) -> bool {
        matches!(
            self.verdict.as_deref(),
            Some("checked" | "checked_with_sorry")
        )
    }
}

/// The results of one rung over the whole set.
#[derive(Debug, Clone)]
pub struct BenchReport {
    /// The rung that was measured.
    pub rung: String,
    /// One result per case, in case order.
    pub results: Vec<CaseResult>,
}

impl BenchReport {
    /// How many cases compiled.
    #[must_use]
    pub fn checked(&self) -> usize {
        self.results.iter().filter(|r| r.checked()).count()
    }

    /// The share of cases that compiled, `0.0` for an empty report.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counts are far below 2^52
    pub fn rate(&self) -> f64 {
        if self.results.is_empty() {
            0.0
        } else {
            self.checked() as f64 / self.results.len() as f64
        }
    }

    /// A one-line account: the rung, the rate, and the tokens spent.
    #[must_use]
    pub fn summary(&self) -> String {
        let spent = |pick: fn(&Usage) -> u64| -> u64 {
            self.results
                .iter()
                .filter_map(|r| r.usage.as_ref())
                .map(pick)
                .sum()
        };
        format!(
            "{}: {}/{} checked ({:.1}%), {} input and {} output tokens",
            self.rung,
            self.checked(),
            self.results.len(),
            self.rate() * 100.0,
            spent(|u| u.input_tokens),
            spent(|u| u.output_tokens),
        )
    }
}

/// Runs `cases` through an `agent_formalize` tool, one call each.
///
/// `ctx` supplies the project and budget; each call gets its own call id.
pub async fn run_formalize(
    tool: &dyn Tool,
    ctx: &CallContext,
    rung: &str,
    cases: &[FormalizeCase],
) -> BenchReport {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let mut call_ctx = ctx.clone();
        call_ctx.call_id = format!("{}.{}", ctx.call_id, case.id);
        let envelope = tool
            .call(
                &call_ctx,
                json!({"source": {"statement": case.statement}, "path": case.path()}),
            )
            .await;
        results.push(CaseResult {
            id: case.id.clone(),
            outcome: envelope.outcome,
            verdict: envelope
                .data
                .get("verdict")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            error: envelope.code(),
            usage: envelope.usage,
        });
    }
    BenchReport {
        rung: rung.to_owned(),
        results,
    }
}

#[cfg(test)]
mod test;
