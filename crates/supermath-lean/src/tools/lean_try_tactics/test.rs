//! Unit tests for the tactic tool's pure helpers.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::{Position, edit_call, tidy};
use crate::client::TacticResult;

fn at(line: u32, column: u32) -> Position {
    Position { line, column }
}

#[test]
fn the_edit_call_replaces_only_the_sorry() {
    let text = "theorem t : 1 = 1 := by\n  sorry\n";
    let call = edit_call("lean/Supermath/A.lean", text, "abc", &at(2, 2), "rfl").unwrap();
    let json: serde_json::Value =
        serde_json::from_str(call.strip_prefix("file_edit ").unwrap()).unwrap();
    assert_eq!(json["edits"][0]["old"], "  sorry");
    assert_eq!(json["edits"][0]["new"], "  rfl");
    assert_eq!(json["if_sha256"], "abc");
}

#[test]
fn a_sorry_not_at_the_column_is_still_replaced_in_its_line() {
    let text = "theorem t : True := by sorry\n";
    let call = edit_call("p", text, "s", &at(1, 0), "trivial").unwrap();
    assert!(call.contains("theorem t : True := by trivial"));
}

#[test]
fn no_call_is_built_when_the_line_repeats_or_is_missing() {
    let text = "  sorry\n  sorry\n";
    assert!(edit_call("p", text, "s", &at(1, 2), "rfl").is_none());
    assert!(edit_call("p", text, "s", &at(9, 0), "rfl").is_none());
    assert!(edit_call("p", "\n", "s", &at(1, 0), "rfl").is_none());
}

#[test]
fn an_unknown_outcome_becomes_a_failure() {
    let tidy_result = tidy(TacticResult {
        tactic: "simp".into(),
        result: "weird".into(),
        ..TacticResult::default()
    });
    assert_eq!(tidy_result.result, "fails");
    assert!(tidy_result.error.unwrap().contains("weird"));
}

#[test]
fn long_errors_and_goals_are_trimmed() {
    let tidy_result = tidy(TacticResult {
        tactic: "simp".into(),
        result: "fails".into(),
        error: Some("e".repeat(900)),
        goals: vec!["g".repeat(3_000)],
        ..TacticResult::default()
    });
    assert_eq!(tidy_result.error.unwrap().chars().count(), 401);
    assert_eq!(tidy_result.goals[0].chars().count(), 2_001);
}
