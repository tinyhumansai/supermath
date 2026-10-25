//! `supermath-mcp score proof`: the rsi-ralph scorer of a conjecture loop.
//!
//! rsi-ralph runs a scorer once per seed, in the loop folder, with
//! `RSI_CANDIDATE` naming the checkout to judge and `RSI_OUT` an empty
//! directory for its outputs (`docs/specs/rsi-cli.md`, "Step contract"
//! there). This one never trusts a seat's account of its work, and never
//! writes into the candidate:
//!
//! 1. [`prepare`] copies the candidate's `lean/` and `python/` into
//!    `$RSI_OUT/project` (no `.lake` build folder, no symlinks), and writes a
//!    host-owned verdict file per side whose theorem file exists:
//!    `ScoreProved.lean` holds `theorem settled_proved : Conjecture := proof`,
//!    `ScoreRefuted.lean` `theorem settled_refuted : ¬ Conjecture := refute`.
//!    Each type-checks only if the side's theorem has exactly the pinned type.
//! 2. `lean_check` runs on each side's own file, then on its verdict file,
//!    against the copy (`--project-dir` semantics, through
//!    [`Toolkit::compose_persistent`](crate::Toolkit::compose_persistent)).
//! 3. [`judge`] turns the envelopes into a [`Report`]: `1.0` when a verdict
//!    file is `checked` with only the standard axioms, `0.75` when it is
//!    `checked` with named debt (`native_decide`), otherwise the best side's
//!    partial credit: `0.2 + 0.3 / (1 + sorries)` for a file that
//!    elaborates (so any such file beats any that does not), and
//!    `0.1 / (1 + errors)` for one that does not.
//!
//! The report is written as `$RSI_OUT/result.json` (`score`, `failures` the
//! next brief shows, `metrics`) and printed as `METRIC proof=<score>`.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use supermath_tool::{Budget, Envelope, Outcome};

use crate::error::{Error, Result};
use crate::toolkit::Toolkit;

/// How long one scorer check may run: building a side's imported lemmas comes
/// first, so this is far above an interactive check's limit.
pub const CHECK_TIMEOUT_MS: u64 = 600_000;

/// The least a file that elaborates scores: above anything a file with
/// errors can.
const ELABORATES_FLOOR: f64 = 0.2;

/// What a file that elaborates earns above the floor, shared out by its
/// `sorry`s: all of it with none.
const ELABORATES_SPAN: f64 = 0.3;

/// Most failure entries a report carries per side.
const FAILURES_PER_SIDE: usize = 5;

/// One side of the conjecture: the file a seat writes, and the host-written
/// file that checks it against the pinned statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Side {
    /// `prove` or `refute`, as metric names use it.
    pub name: &'static str,
    /// The seat's file.
    pub file: &'static str,
    /// The theorem the seat's file must declare, fully qualified.
    pub theorem: &'static str,
    /// The host-written verdict file.
    pub verdict_file: &'static str,
    /// The verdict file's theorem, fully qualified.
    pub verdict_theorem: &'static str,
    /// The verdict file's text.
    verdict_text: &'static str,
}

/// The prover's side, then the refuter's.
pub const SIDES: [Side; 2] = [
    Side {
        name: "prove",
        file: "lean/Supermath/Prove.lean",
        theorem: "Supermath.proof",
        verdict_file: "lean/Supermath/ScoreProved.lean",
        verdict_theorem: "Supermath.settled_proved",
        verdict_text: "import Supermath.Statement\nimport Supermath.Prove\n\nnamespace Supermath\n\n\
                       /-- Written by the scorer: checks only if `proof` proves the pinned `Conjecture`. -/\n\
                       theorem settled_proved : Conjecture := proof\n\nend Supermath\n",
    },
    Side {
        name: "refute",
        file: "lean/Supermath/Refute.lean",
        theorem: "Supermath.refute",
        verdict_file: "lean/Supermath/ScoreRefuted.lean",
        verdict_theorem: "Supermath.settled_refuted",
        verdict_text: "import Supermath.Statement\nimport Supermath.Refute\n\nnamespace Supermath\n\n\
                       /-- Written by the scorer: checks only if `refute` refutes the pinned `Conjecture`. -/\n\
                       theorem settled_refuted : ¬ Conjecture := refute\n\nend Supermath\n",
    },
];

/// What a scoring run found.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The candidate's score, in `[0, 1]`.
    pub score: f64,
    /// The side whose verdict file checked, if any.
    pub settled: Option<&'static str>,
    /// What the next brief should show: Lean's errors and open goals.
    pub failures: Vec<Value>,
    /// Counts per side, and `settled` as 0 or 1.
    pub metrics: Map<String, Value>,
}

impl Report {
    /// The `result.json` rsi-ralph reads.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "score": self.score,
            "failures": self.failures,
            "metrics": self.metrics,
        })
    }
}

/// One side's two checks: its own file, then its verdict file. `None` when
/// the side's file does not exist.
pub type Checks = Option<(Envelope, Envelope)>;

/// Copies `candidate` into `out/project` and writes the verdict file of
/// every side whose file exists. Returns the copy's directory.
///
/// # Errors
///
/// [`Error::Io`] if the candidate cannot be read or the copy written.
pub fn prepare(candidate: &Path, out: &Path) -> Result<PathBuf> {
    let project = out.join("project");
    for top in ["lean", "python"] {
        let from = candidate.join(top);
        if from.is_dir() {
            copy_tree(&from, &project.join(top))?;
        }
    }
    for side in SIDES {
        if project.join(side.file).is_file() {
            write(&project.join(side.verdict_file), side.verdict_text)?;
        }
    }
    Ok(project)
}

/// Runs both checks of every side present in `toolkit`'s project.
pub async fn check(toolkit: &Toolkit) -> [Checks; 2] {
    let mut checks: [Checks; 2] = [None, None];
    for (index, side) in SIDES.iter().enumerate() {
        let main = run(
            toolkit,
            &format!("score-{}-file", side.name),
            side.file,
            None,
        )
        .await;
        if main.code() == Some(supermath_tool::ErrorCode::NotFound) {
            continue;
        }
        let verdict = run(
            toolkit,
            &format!("score-{}-verdict", side.name),
            side.verdict_file,
            Some(side.verdict_theorem),
        )
        .await;
        checks[index] = Some((main, verdict));
    }
    checks
}

async fn run(toolkit: &Toolkit, call_id: &str, path: &str, target: Option<&str>) -> Envelope {
    let budget = Budget {
        wall_time_ms: CHECK_TIMEOUT_MS,
        ..Budget::default()
    };
    let ctx = toolkit.context(call_id).with_budget(budget);
    let mut args = json!({"path": path});
    if let Some(target) = target {
        args["targets"] = json!([target]);
    }
    toolkit.dispatch_with(&ctx, "lean_check", args).await
}

/// The report the checks of both sides earn.
#[must_use]
pub fn judge(checks: &[Checks; 2]) -> Report {
    let mut report = Report {
        score: 0.0,
        settled: None,
        failures: Vec::new(),
        metrics: Map::new(),
    };
    for (side, checks) in SIDES.iter().zip(checks) {
        let Some((main, verdict)) = checks else {
            continue;
        };
        let (score, settled) = side_score(main, verdict);
        if settled && report.settled.is_none() {
            report.settled = Some(side.name);
        }
        report.score = report.score.max(score);
        let data = &main.data;
        report.metrics.insert(
            format!("{}_sorries", side.name),
            json!(data["sorries"].as_array().map_or(0, Vec::len)),
        );
        report.metrics.insert(
            format!("{}_errors", side.name),
            json!(data["error_count"].as_u64().unwrap_or(0)),
        );
        if !settled {
            report.failures.extend(failures(side, main, verdict));
        }
    }
    report.metrics.insert(
        "settled".to_owned(),
        json!(u8::from(report.settled.is_some())),
    );
    report
}

/// A side's score, and whether its verdict file settled the conjecture.
fn side_score(main: &Envelope, verdict: &Envelope) -> (f64, bool) {
    if verdict.outcome == Outcome::Ok && verdict.data["verdict"] == "checked" {
        let debt = verdict.data["provenance"]["debt"]
            .as_array()
            .is_some_and(|debt| !debt.is_empty());
        return if debt { (0.75, false) } else { (1.0, true) };
    }
    let data = &main.data;
    let count = |n: usize| f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    match data["verdict"].as_str() {
        Some("checked") => (ELABORATES_FLOOR + ELABORATES_SPAN, false),
        Some("checked_with_sorry") => {
            let sorries = data["sorries"].as_array().map_or(1, Vec::len);
            (
                ELABORATES_FLOOR + ELABORATES_SPAN / (1.0 + count(sorries)),
                false,
            )
        }
        Some("failed") => {
            let errors = data["error_count"].as_u64().unwrap_or(1);
            let errors = usize::try_from(errors).unwrap_or(usize::MAX);
            (0.1 / (1.0 + count(errors)), false)
        }
        _ => (0.0, false),
    }
}

/// What a side should hear about: its own file's errors and open goals, and
/// when that file is clean, why the verdict file still did not check.
fn failures(side: &Side, main: &Envelope, verdict: &Envelope) -> Vec<Value> {
    let mut out = Vec::new();
    let file = side.file;
    for error in main.data["errors"].as_array().into_iter().flatten() {
        out.push(json!({
            "id": format!("{}-error-{}", side.name, out.len() + 1),
            "type": "lean_error",
            "text": format!("{file}:{}:{}: {}", error["line"], error["column"],
                error["message"].as_str().unwrap_or_default()),
        }));
    }
    for sorry in main.data["sorries"].as_array().into_iter().flatten() {
        out.push(json!({
            "id": format!("{}-sorry-{}", side.name, out.len() + 1),
            "type": "open_goal",
            "text": format!("{file}:{}:{}: sorry with goal {}", sorry["line"], sorry["column"],
                sorry["goal"].as_str().unwrap_or("?")),
        }));
    }
    if main.outcome != Outcome::Ok {
        if let Some(error) = &main.error {
            out.push(json!({
                "id": format!("{}-check", side.name),
                "type": "check_failed",
                "text": format!("lean_check {file}: {}", error.message),
            }));
        }
    } else if main.data["verdict"] == "checked" {
        let reason = verdict.data["errors"][0]["message"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| verdict.error.as_ref().map(|error| error.message.clone()))
            .unwrap_or_else(|| "it did not check".to_owned());
        out.push(json!({
            "id": format!("{}-type", side.name),
            "type": "wrong_statement",
            "text": format!("{file} checks, but `{}` does not have the pinned type: {reason}",
                side.theorem),
        }));
    }
    out.truncate(FAILURES_PER_SIDE);
    out
}

/// Writes `report` as `out/result.json` and returns the metric line.
///
/// # Errors
///
/// [`Error::Io`] if the file cannot be written.
pub fn write_result(out: &Path, report: &Report) -> Result<String> {
    write(&out.join("result.json"), &report.to_json().to_string())?;
    Ok(format!("METRIC proof={}", report.score))
}

/// Copies the files under `from` into `to`, skipping symlinks and `.lake`.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let io = |path: &Path| {
        let path = path.display().to_string();
        move |source| Error::Io { path, source }
    };
    std::fs::create_dir_all(to).map_err(io(to))?;
    for entry in std::fs::read_dir(from).map_err(io(from))? {
        let entry = entry.map_err(io(from))?;
        let kind = entry.file_type().map_err(io(&entry.path()))?;
        let target = to.join(entry.file_name());
        if kind.is_dir() && entry.file_name() != ".lake" {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target).map_err(io(&target))?;
        }
    }
    Ok(())
}

fn write(path: &Path, text: &str) -> Result<()> {
    let io = |source| Error::Io {
        path: path.display().to_string(),
        source,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    std::fs::write(path, text).map_err(io)
}

#[cfg(test)]
mod test;
