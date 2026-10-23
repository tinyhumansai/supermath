//! Unit tests for the solve loop's reporting and its stop rule.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use openhuman_embed::AgentProgress;
use serde_json::json;
use supermath_tool::{CallLog, CallRecord, Envelope, ErrorCode, InMemoryCallLog};

use super::{CONTINUE_PROMPT, Transcript, call_line, declares_done, excerpt, progress_line, sweep};

#[test]
fn a_later_turn_tells_the_model_to_load_its_tools_again() {
    // Each turn starts with only the direct tools; what `tool_search` loaded
    // in an earlier turn is gone. A model that calls a tool it has not loaded
    // again sends a different, loaded tool instead (a live run called
    // `ledger_record` 80 times meaning `agent_review_alignment`).
    assert!(CONTINUE_PROMPT.contains("tool_search"));
    assert!(CONTINUE_PROMPT.contains("earlier turn"));
}

#[test]
fn a_reply_is_done_only_with_the_marker_on_its_own_line() {
    assert!(declares_done(
        "No counterexample below 10^6.\n\nSTATUS: DONE\n"
    ));
    assert!(declares_done("  STATUS: DONE  "));
    assert!(!declares_done("I will write STATUS: DONE when finished."));
    assert!(!declares_done(""));
}

#[test]
fn a_transcript_is_done_when_its_last_reply_says_so() {
    let mut transcript = Transcript {
        session_id: "s1".into(),
        replies: vec![],
    };
    assert!(!transcript.done());
    transcript.replies.push("STATUS: DONE".into());
    transcript.replies.push("more to do".into());
    assert!(!transcript.done());
    transcript.replies.push("settled\nSTATUS: DONE".into());
    assert!(transcript.done());
}

#[test]
fn tool_calls_and_iterations_get_a_line_and_other_events_none() {
    let started = AgentProgress::ToolCallStarted {
        call_id: "c1".into(),
        tool_name: "calc_factor".into(),
        arguments: json!({ "n": 30 }),
        iteration: 1,
        display_label: None,
        display_detail: None,
    };
    assert_eq!(
        progress_line(&started).unwrap(),
        r#"-> calc_factor {"n":30}"#
    );
    let iteration = AgentProgress::IterationStarted {
        iteration: 2,
        max_iterations: 24,
    };
    assert_eq!(progress_line(&iteration).unwrap(), "-- iteration 2/24");
    assert_eq!(
        progress_line(&AgentProgress::ContextCompacting).unwrap(),
        "-- compacting context"
    );
    assert!(progress_line(&AgentProgress::TurnStarted).is_none());
}

#[test]
fn a_completed_call_reports_its_status_and_an_excerpt() {
    let completed = |success| AgentProgress::ToolCallCompleted {
        call_id: "c1".into(),
        tool_name: "lean_check".into(),
        success,
        output_chars: 9,
        output: "verdict:\n  checked".into(),
        arguments: None,
        elapsed_ms: 1200,
        iteration: 1,
        failure: None,
        display_label: None,
        display_detail: None,
        structured: None,
    };
    assert_eq!(
        progress_line(&completed(true)).unwrap(),
        "<- lean_check ok (1200 ms) verdict: checked"
    );
    assert!(
        progress_line(&completed(false))
            .unwrap()
            .contains(" failed ")
    );
}

#[test]
fn long_text_is_cut_to_one_short_line() {
    let long = "word ".repeat(200);
    let cut = excerpt(&long);
    assert!(cut.ends_with('…'));
    assert_eq!(cut.chars().count(), super::EXCERPT_CHARS + 1);
    assert_eq!(excerpt("a\n b"), "a b");
}

#[test]
fn a_call_line_names_the_call_its_outcome_and_any_code() {
    let ok = CallRecord::new(
        "agent-1-p3",
        "lean_check",
        "erdos",
        &Envelope::ok(json!({})),
        40,
    );
    assert_eq!(call_line(&ok), "   log agent-1-p3 lean_check -> Ok (40 ms)");
    let refused = CallRecord::new(
        "c2",
        "file_write",
        "erdos",
        &Envelope::error(ErrorCode::Forbidden, "outside the area"),
        1,
    );
    assert_eq!(
        call_line(&refused),
        "   log c2 file_write -> Error Forbidden (1 ms)"
    );
}

#[test]
fn each_sweep_reports_only_records_it_has_not_reported() {
    let log = InMemoryCallLog::new();
    let reported = AtomicUsize::new(0);
    let lines = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let lines = Arc::clone(&lines);
        move |line: String| lines.lock().unwrap().push(line)
    };
    let record = |id: &str| CallRecord::new(id, "calc_gcd_lcm", "p", &Envelope::ok(json!({})), 0);

    log.record(record("a"));
    log.record(record("b"));
    sweep(&log, &reported, &sink);
    sweep(&log, &reported, &sink);
    log.record(record("c"));
    sweep(&log, &reported, &sink);

    let lines = lines.lock().unwrap();
    assert_eq!(lines.len(), 3);
    assert!(lines[2].contains(" c calc_gcd_lcm"));
}
