//! Unit tests for the harness prompts.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::prompt::{SOLVER_PROMPT, memory_note, turn_message};
use crate::toolkit::RECALL_TOOL;
use crate::problem::Problem;
use crate::verdict::{Reason, Verdict};

fn problem() -> Problem {
    Problem::new("odd-sums", "Show 1+3+...+(2n-1) = n^2.").unwrap()
}

#[test]
fn solver_prompt_asks_for_each_verdict_line_the_parser_reads() {
    for line in [
        "VERDICT: proved",
        "VERDICT: refuted",
        "VERDICT: not-established",
    ] {
        assert!(SOLVER_PROMPT.contains(line), "{line}");
        assert_ne!(
            Verdict::parse(line),
            Verdict::NotEstablished(Reason::NoVerdict)
        );
    }
    assert!(SOLVER_PROMPT.contains(RECALL_TOOL));
}

#[test]
fn turn_message_carries_the_statement_and_says_when_memory_is_empty() {
    let message = turn_message(&problem(), None);
    assert!(message.contains("## Problem `odd-sums`"));
    assert!(message.contains("n^2"));
    assert!(message.contains("Memory holds nothing"));

    let message = turn_message(&problem(), Some("1. checked n<=5"));
    assert!(message.contains("1. checked n<=5"));
    assert!(!message.contains("Memory holds nothing"));
}

#[test]
fn memory_note_leads_with_the_verdict_and_clips_a_long_reply() {
    let note = memory_note(&Verdict::Proved, "  proof  ");
    assert_eq!(note, "Run verdict: proved\n\nproof");

    let note = memory_note(&Verdict::Refuted, &"y".repeat(10_000));
    assert!(note.starts_with("Run verdict: refuted"));
    assert!(note.ends_with('…'));
    assert!(note.chars().count() < 6_100);
}
