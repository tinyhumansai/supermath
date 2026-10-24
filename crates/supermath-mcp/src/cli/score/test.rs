//! Scoring a conjecture loop's candidate: the copy, the checks, the score.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use supermath_tool::{Envelope, ErrorCode};

use super::{Checks, Report, SIDES, judge, prepare, write_result};

fn data(verdict: &str, errors: usize, sorries: usize, debt: &[&str]) -> Value {
    json!({
        "verdict": verdict,
        "error_count": errors,
        "errors": (0..errors).map(|i| json!({"line": i + 1, "column": 2, "message": format!("bad {i}")})).collect::<Vec<_>>(),
        "sorries": (0..sorries).map(|i| json!({"line": i + 1, "column": 4, "goal": "⊢ P"})).collect::<Vec<_>>(),
        "provenance": {"debt": debt},
    })
}

fn ok(value: Value) -> Envelope {
    Envelope::ok(value)
}

fn side(main: Envelope, verdict: Envelope) -> Checks {
    Some((main, verdict))
}

fn score(prove: Checks, refute: Checks) -> Report {
    judge(&[prove, refute])
}

#[test]
fn a_clean_verdict_file_settles_the_conjecture() {
    let report = score(
        side(ok(data("checked", 0, 0, &[])), ok(data("checked", 0, 0, &[]))),
        None,
    );
    assert_eq!(report.score, 1.0);
    assert_eq!(report.settled, Some("prove"));
    assert_eq!(report.metrics["settled"], 1);
    assert_eq!(report.failures, Vec::<Value>::new());
}

#[test]
fn a_refutation_settles_it_too() {
    let report = score(
        side(ok(data("failed", 2, 0, &[])), ok(data("failed", 1, 0, &[]))),
        side(ok(data("checked", 0, 0, &[])), ok(data("checked", 0, 0, &[]))),
    );
    assert_eq!((report.score, report.settled), (1.0, Some("refute")));
    // The unsettled prover's errors still reach the next brief.
    assert_eq!(report.failures.len(), 2);
}

#[test]
fn named_debt_scores_below_settled() {
    let report = score(
        side(ok(data("checked", 0, 0, &[])), ok(data("checked", 0, 0, &["Supermath.settled_proved"]))),
        None,
    );
    assert_eq!((report.score, report.settled), (0.75, None));
    assert_eq!(report.metrics["settled"], 0);
}

#[test]
fn fewer_sorries_and_fewer_errors_score_higher() {
    let partial = |verdict: &str, errors, sorries| {
        score(
            side(ok(data(verdict, errors, sorries, &[])), ok(data("checked_with_sorry", 0, 1, &[]))),
            None,
        )
        .score
    };
    assert_eq!(partial("checked_with_sorry", 0, 1), 0.25);
    assert!(partial("checked_with_sorry", 0, 1) > partial("checked_with_sorry", 0, 3));
    assert!(partial("checked_with_sorry", 0, 9) > partial("failed", 1, 0));
    assert!(partial("failed", 1, 0) > partial("failed", 4, 0));
    assert_eq!(partial("unknown", 0, 0), 0.0);
}

#[test]
fn the_best_side_counts_and_each_reports_its_own_metrics() {
    let report = score(
        side(ok(data("failed", 3, 0, &[])), ok(data("failed", 1, 0, &[]))),
        side(ok(data("checked_with_sorry", 0, 1, &[])), ok(data("checked_with_sorry", 0, 1, &[]))),
    );
    assert_eq!(report.score, 0.25);
    assert_eq!(report.metrics["prove_errors"], 3);
    assert_eq!(report.metrics["refute_sorries"], 1);
    let ids: Vec<&str> = report.failures.iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["prove-error-1", "prove-error-2", "prove-error-3", "refute-sorry-1"]);
    assert_eq!(report.failures[3]["text"], "lean/Supermath/Refute.lean:1:4: sorry with goal ⊢ P");
}

#[test]
fn a_clean_file_whose_theorem_has_another_type_says_so() {
    let verdict = json!({"verdict": "failed", "error_count": 1,
        "errors": [{"line": 7, "column": 0, "message": "type mismatch"}], "sorries": []});
    let report = score(side(ok(data("checked", 0, 0, &[])), ok(verdict)), None);
    assert_eq!(report.score, 0.5);
    assert_eq!(report.failures[0]["type"], "wrong_statement");
    assert!(report.failures[0]["text"].as_str().unwrap().contains("type mismatch"));
    // Without a message, the verdict envelope's own error is quoted.
    let refused = Envelope::error(ErrorCode::Timeout, "too slow");
    let report = score(side(ok(data("checked", 0, 0, &[])), refused), None);
    assert!(report.failures[0]["text"].as_str().unwrap().contains("too slow"));
}

#[test]
fn a_check_that_failed_to_run_is_a_failure_and_scores_nothing() {
    let down = Envelope::error(ErrorCode::BackendUnavailable, "lean is unavailable");
    let report = score(side(down.clone(), down), None);
    assert_eq!(report.score, 0.0);
    assert_eq!(report.failures[0]["type"], "check_failed");
}

#[test]
fn no_side_file_scores_zero_and_failures_are_capped() {
    let report = score(None, None);
    assert_eq!((report.score, report.settled), (0.0, None));
    assert_eq!(report.metrics.len(), 1);
    let report = score(side(ok(data("failed", 9, 0, &[])), ok(data("failed", 1, 0, &[]))), None);
    assert_eq!(report.failures.len(), 5);
}

#[test]
fn prepare_copies_the_sources_and_writes_a_verdict_per_present_side() {
    let root = tempfile::tempdir().unwrap();
    let candidate = root.path().join("candidate");
    let put = |path: &str, text: &str| {
        let path = candidate.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    put(SIDES[0].file, "theorem proof : Conjecture := sorry\n");
    put("lean/Supermath/Statement.lean", "def Conjecture : Prop := True\n");
    put("lean/.lake/build/junk", "x");
    put("python/work/search.py", "def f():\n    return 1\n");
    put("notes/prover.md", "notes stay behind");
    #[cfg(unix)]
    std::os::unix::fs::symlink("/etc/passwd", candidate.join("lean/Supermath/Link.lean")).unwrap();

    let out = root.path().join("out");
    let project = prepare(&candidate, &out).unwrap();
    assert_eq!(project, out.join("project"));
    assert!(project.join(SIDES[0].file).is_file());
    assert!(project.join("python/work/search.py").is_file());
    assert!(!project.join("lean/.lake").exists());
    assert!(!project.join("notes").exists());
    assert!(!project.join("lean/Supermath/Link.lean").exists());
    let verdict = std::fs::read_to_string(project.join(SIDES[0].verdict_file)).unwrap();
    assert!(verdict.contains("theorem settled_proved : Conjecture := proof"));
    assert!(!project.join(SIDES[1].verdict_file).exists(), "no Refute.lean, no verdict");
}

#[test]
fn prepare_fails_on_an_unwritable_output() {
    let root = tempfile::tempdir().unwrap();
    let candidate = root.path().join("candidate");
    std::fs::create_dir_all(candidate.join("lean")).unwrap();
    let blocker = root.path().join("blocker");
    std::fs::write(&blocker, "").unwrap();
    assert!(matches!(
        prepare(&candidate, &blocker),
        Err(crate::Error::Io { .. })
    ));
}

#[test]
fn the_result_is_the_json_rsi_ralph_reads_and_a_metric_line() {
    let root = tempfile::tempdir().unwrap();
    let report = score(None, None);
    let line = write_result(root.path(), &report).unwrap();
    assert_eq!(line, "METRIC proof=0");
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(root.path().join("result.json")).unwrap())
            .unwrap();
    assert_eq!(written, json!({"score": 0.0, "failures": [], "metrics": {"settled": 0}}));
}
