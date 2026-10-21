//! Unit tests for the harness prompts and the `recall_memory` tool.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use openhuman_embed::Tool;
use serde_json::json;

use super::prompt::{SOLVER_PROMPT, memory_note, turn_message};
use super::tool::{RECALL_TOOL, RecallTool};
use crate::memory::Memory;
use crate::problem::Problem;
use crate::verdict::{Reason, Verdict};

fn problem() -> Problem {
    Problem::new("odd-sums", "Show 1+3+...+(2n-1) = n^2.").unwrap()
}

/// A memory client pointed at a port nothing listens on.
async fn unreachable_memory() -> Memory {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    Memory::new(url, "k", "ws:t")
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

#[tokio::test]
async fn recall_tool_rejects_a_missing_or_blank_query() {
    let tool = RecallTool::new(unreachable_memory().await, problem());
    for arguments in [json!({}), json!({ "query": "  " }), json!({ "query": 3 })] {
        let result = tool.execute(arguments).await.unwrap();
        assert!(result.output().contains("non-empty"), "{}", result.output());
    }
}

#[tokio::test]
async fn recall_tool_reports_unavailable_memory_to_the_model() {
    let tool = RecallTool::new(unreachable_memory().await, problem());
    let result = tool.execute(json!({ "query": "odd" })).await.unwrap();
    assert!(
        result.output().contains("memory is unavailable"),
        "{}",
        result.output()
    );
}

#[test]
fn recall_tool_is_read_only_and_requires_a_query() {
    let tool = RecallTool::new(Memory::new("http://x", "k", "ws:t"), problem());
    assert_eq!(tool.name(), RECALL_TOOL);
    assert!(tool.description().contains("recorded"));
    assert_eq!(tool.parameters_schema()["required"], json!(["query"]));
}
