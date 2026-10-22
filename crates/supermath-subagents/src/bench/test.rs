//! Tests for the benchmark set and its runner.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use serde_json::json;
use supermath_tool::Outcome;

use super::{
    ArxivCase, BenchCase, BenchReport, CaseResult, FormalizeCase, arxiv_cases, formalize_cases,
    run_formalize,
};
use crate::fake::{ScriptRunner, World, ctx, script, subagents, tool, turn};

#[test]
fn the_checked_in_set_has_thirty_distinct_complete_cases() {
    let cases = formalize_cases().unwrap();
    assert_eq!(cases.len(), 30);
    let ids: HashSet<_> = cases.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), 30, "ids are unique");
    for case in &cases {
        assert!(
            case.statement.split_whitespace().count() >= 6,
            "{}",
            case.id
        );
        assert!(
            !case.area.is_empty() && !case.expected.is_empty(),
            "{}",
            case.id
        );
        assert!(!case.expected.contains(char::is_whitespace), "{}", case.id);
        assert!(case.statement.ends_with('.'), "{}", case.id);
        assert!(
            !["TODO", "TBD", "placeholder", "…"]
                .iter()
                .any(|p| case.statement.contains(p)),
            "{}",
            case.id
        );
    }
}

#[test]
fn each_case_gets_a_valid_distinct_lean_path() {
    let cases = formalize_cases().unwrap();
    let paths: HashSet<_> = cases.iter().map(FormalizeCase::path).collect();
    assert_eq!(paths.len(), 30);
    for path in &paths {
        assert!(crate::tools::valid_lean_path_for_tests(path), "{path}");
    }
    let one = cases.iter().find(|c| c.id == "even-add").unwrap();
    assert_eq!(one.path(), "lean/Supermath/Bench/EvenAdd.lean");
}

#[test]
fn the_arxiv_set_has_twelve_distinct_version_pinned_cases() {
    let cases = arxiv_cases().unwrap();
    assert_eq!(cases.len(), 12);
    let ids: HashSet<_> = cases.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids.len(), 12, "ids are unique");
    let sources: HashSet<_> = cases.iter().map(|c| (&c.arxiv_id, &c.label)).collect();
    assert_eq!(sources.len(), 12, "each result is asked for once");
    for case in &cases {
        let (_, version) = case.arxiv_id.rsplit_once('v').unwrap();
        assert!(
            !version.is_empty() && version.bytes().all(|b| b.is_ascii_digit()),
            "{} pins a version",
            case.id
        );
        assert!(
            crate::tools::valid_arxiv_id_for_tests(&case.arxiv_id),
            "{}",
            case.id
        );
        assert!(
            !case.area.is_empty() && !case.label.is_empty(),
            "{}",
            case.id
        );
    }
}

#[test]
fn every_case_in_both_sets_gets_its_own_lean_path() {
    let mut paths: Vec<String> = formalize_cases()
        .unwrap()
        .iter()
        .map(BenchCase::path)
        .collect();
    paths.extend(arxiv_cases().unwrap().iter().map(BenchCase::path));
    let distinct: HashSet<_> = paths.iter().collect();
    assert_eq!(distinct.len(), 42);
    for path in &paths {
        assert!(crate::tools::valid_lean_path_for_tests(path), "{path}");
    }
}

#[tokio::test]
async fn the_runner_formalizes_an_arxiv_case_by_its_label() {
    let world = World::new();
    world.add_paper("2503.06069v3", &["lemm:str", "lemm:leg"]);
    let runner = ScriptRunner::new(script(|prompt, belt| async move {
        assert!(prompt.contains("lemm:leg"), "{prompt}");
        let path = prompt
            .split("Target file: ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .unwrap()
            .to_owned();
        belt.call(
            "file_write",
            json!({"path": path, "content": "theorem t : True := by\n  sorry\n"}),
        )
        .await;
        Ok(turn("NOTE: none"))
    }));
    let built = subagents(&world.registry(), &runner);
    let cases = [
        ArxivCase {
            id: "legendre-formula".into(),
            area: "number-theory".into(),
            arxiv_id: "2503.06069v3".into(),
            label: "lemm:leg".into(),
        },
        ArxivCase {
            id: "missing-label".into(),
            area: "number-theory".into(),
            arxiv_id: "2503.06069v3".into(),
            label: "lemm:nowhere".into(),
        },
    ];
    let report = run_formalize(
        tool(&built, "agent_formalize").as_ref(),
        &ctx(),
        "scripted",
        &cases,
    )
    .await;
    let found = &report.results[0];
    assert_eq!(found.id, "legendre-formula");
    assert!(found.checked(), "{found:?}");
    let missing = &report.results[1];
    assert_eq!(missing.id, "missing-label");
    assert!(!missing.checked());
    assert_eq!(missing.outcome, Outcome::Error, "{missing:?}");
    assert_eq!(report.checked(), 1);
}

#[tokio::test]
async fn the_runner_counts_cases_whose_statement_compiles() {
    let world = World::new();
    world.set_verdict("lean/Supermath/Bench/Cantor.lean", "failed");
    let runner = ScriptRunner::new(script(|prompt, belt| async move {
        let path = prompt
            .split("Target file: ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .unwrap()
            .to_owned();
        belt.call(
            "file_write",
            json!({"path": path, "content": "theorem t : True := by\n  sorry\n"}),
        )
        .await;
        Ok(turn("NOTE: none"))
    }));
    let built = subagents(&world.registry(), &runner);
    let cases = formalize_cases().unwrap();
    let report = run_formalize(
        tool(&built, "agent_formalize").as_ref(),
        &ctx(),
        "scripted",
        &cases,
    )
    .await;
    assert_eq!(report.results.len(), 30);
    assert_eq!(report.checked(), 29);
    let failed = report.results.iter().find(|r| !r.checked()).unwrap();
    assert_eq!(
        (failed.id.as_str(), failed.verdict.as_deref()),
        ("cantor", Some("failed"))
    );
    assert_eq!(failed.outcome, Outcome::Ok);
    assert!((report.rate() - 29.0 / 30.0).abs() < 1e-9);
    assert_eq!(
        report.summary(),
        "scripted: 29/30 checked (96.7%), 3000 input and 600 output tokens"
    );
}

#[test]
fn an_empty_report_has_a_zero_rate_and_a_refusal_is_not_checked() {
    let report = BenchReport {
        rung: "none".into(),
        results: Vec::new(),
    };
    assert!(report.rate().abs() < f64::EPSILON);
    assert_eq!(
        report.summary(),
        "none: 0/0 checked (0.0%), 0 input and 0 output tokens"
    );
    let refused = CaseResult {
        id: "x".into(),
        outcome: Outcome::Error,
        verdict: None,
        error: Some(supermath_tool::ErrorCode::BudgetExceeded),
        usage: None,
    };
    assert!(!refused.checked());
}
