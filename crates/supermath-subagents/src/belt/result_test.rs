//! Unit tests for how an envelope is handed to `OpenHuman`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use supermath_tool::{Envelope, ErrorCode};

use super::tool_result;

fn text(result: &openhuman_embed::ToolResult) -> Value {
    let first = serde_json::to_value(result).unwrap()["content"][0].clone();
    match first.get("data") {
        Some(data) => data.clone(),
        None => serde_json::from_str(first["text"].as_str().unwrap()).unwrap(),
    }
}

#[test]
fn ok_and_unknown_envelopes_are_results() {
    for envelope in [Envelope::ok(json!({"n": 1})), Envelope::unknown("no witness below 10^6")] {
        let body = serde_json::to_value(&envelope).unwrap();
        let result = tool_result(&envelope, body.clone());
        assert!(!result.is_error);
        assert_eq!(text(&result), body);
    }
}

#[test]
fn refusals_are_results_the_model_reads_and_never_failures() {
    for code in [
        ErrorCode::InvalidInput,
        ErrorCode::NotFound,
        ErrorCode::Forbidden,
        ErrorCode::Conflict,
        ErrorCode::BudgetExceeded,
        ErrorCode::Timeout,
    ] {
        let envelope = Envelope::error(code, "refused");
        let body = serde_json::to_value(&envelope).unwrap();
        let result = tool_result(&envelope, body.clone());
        assert!(!result.is_error, "{code:?}");
        assert_eq!(text(&result), body, "{code:?}");
    }
}

#[test]
fn service_failures_are_failures_with_a_status_the_harness_reads() {
    // Without an explicit status, OpenHuman sniffs keywords: a message that
    // says "forbidden" or "timed out" would halt the turn outright.
    for (code, status) in [(ErrorCode::BackendUnavailable, 503), (ErrorCode::RateLimited, 429)] {
        let envelope = Envelope::error(code, "lean service timed out: forbidden");
        let body = serde_json::to_value(&envelope).unwrap();
        let result = tool_result(&envelope, body);
        assert!(result.is_error, "{code:?}");
        let sent = text(&result);
        assert_eq!(sent["status_code"], status, "{code:?}");
        assert_eq!(sent["outcome"], "error");
    }
}
