//! Pins the envelope's wire form and its constructors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{Coverage, DEFAULT_LIMIT, Envelope, ErrorCode, MAX_LIMIT, Outcome, Usage, clamp_limit};

#[test]
fn ok_envelope_has_the_pinned_wire_form() {
    let envelope = Envelope::ok(json!({"n": 1}))
        .with_coverage(Coverage::new(["arxiv"], ["zbmath"]))
        .with_next(["lit_get_paper {}"]);
    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        json!({
            "outcome": "ok",
            "data": {"n": 1},
            "coverage": {"searched": ["arxiv"], "not_covered": ["zbmath"]},
            "unknowns": [],
            "next": ["lit_get_paper {}"],
            "cursor": null
        })
    );
}

#[test]
fn error_envelope_carries_a_screaming_snake_code() {
    let envelope = Envelope::error(ErrorCode::RateLimited, "slow down").with_retry_after_ms(500);
    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        json!({
            "outcome": "error",
            "data": null,
            "unknowns": [],
            "next": [],
            "cursor": null,
            "error": {"code": "RATE_LIMITED", "message": "slow down", "retry_after_ms": 500}
        })
    );
    assert_eq!(envelope.code(), Some(ErrorCode::RateLimited));
}

#[test]
fn every_error_code_serialises_to_its_as_str() {
    let codes = [
        (ErrorCode::InvalidInput, "INVALID_INPUT"),
        (ErrorCode::NotFound, "NOT_FOUND"),
        (ErrorCode::BackendUnavailable, "BACKEND_UNAVAILABLE"),
        (ErrorCode::BudgetExceeded, "BUDGET_EXCEEDED"),
        (ErrorCode::Timeout, "TIMEOUT"),
        (ErrorCode::RateLimited, "RATE_LIMITED"),
        (ErrorCode::Forbidden, "FORBIDDEN"),
        (ErrorCode::Conflict, "CONFLICT"),
    ];
    for (code, wire) in codes {
        assert_eq!(code.as_str(), wire);
        assert_eq!(serde_json::to_value(code).unwrap(), json!(wire));
        let back: ErrorCode = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(back, code);
    }
}

#[test]
fn unknown_envelope_records_why_and_is_not_an_error() {
    let envelope = Envelope::unknown("pdf text only").with_unknown("no latex");
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(envelope.unknowns, ["pdf text only", "no latex"]);
    assert!(envelope.code().is_none());
    assert!(!envelope.is_ok());
    assert!(envelope.is_well_formed());
}

#[test]
fn usage_and_cursor_round_trip() {
    let usage = Usage {
        model: "gemini-pro".into(),
        turns: 2,
        tool_calls: 3,
        input_tokens: 10,
        output_tokens: 20,
        elapsed_ms: 30,
    };
    let envelope = Envelope::ok(json!([1]))
        .with_data(json!([1, 2]))
        .with_cursor("c2")
        .with_usage(usage.clone());
    let value = serde_json::to_value(&envelope).unwrap();
    assert_eq!(
        value["usage"],
        json!({"model": "gemini-pro", "turns": 2, "tool_calls": 3,
               "input_tokens": 10, "output_tokens": 20, "elapsed_ms": 30})
    );
    assert_eq!(value["cursor"], json!("c2"));
    let back: Envelope = serde_json::from_value(value).unwrap();
    assert_eq!(back, envelope);
    assert!(envelope.is_ok());
}

#[test]
fn cached_age_appears_only_when_set() {
    let mut coverage = Coverage::new(["oeis"], []);
    assert!(
        !serde_json::to_string(&coverage)
            .unwrap()
            .contains("cached_age")
    );
    coverage.cached_age = Some(60);
    assert_eq!(
        serde_json::to_value(&coverage).unwrap()["cached_age"],
        json!(60)
    );
}

#[test]
fn next_keeps_at_most_three_calls() {
    let envelope = Envelope::ok(json!(null)).with_next(["a", "b", "c", "d"]);
    assert_eq!(envelope.next, ["a", "b", "c"]);
}

#[test]
fn retry_after_is_ignored_off_errors() {
    let envelope = Envelope::ok(json!(null)).with_retry_after_ms(5);
    assert!(envelope.error.is_none());
}

#[test]
fn an_envelope_with_a_mismatched_error_is_malformed() {
    let mut envelope = Envelope::ok(json!(null));
    envelope.error = Envelope::error(ErrorCode::NotFound, "x").error;
    assert!(!envelope.is_well_formed());
    let mut errored = Envelope::error(ErrorCode::NotFound, "x");
    errored.error = None;
    assert!(!errored.is_well_formed());
    let mut chatty = Envelope::ok(json!(null));
    chatty.next = vec!["1".into(), "2".into(), "3".into(), "4".into()];
    assert!(!chatty.is_well_formed());
}

#[test]
fn sparse_json_deserialises_with_defaults() {
    let envelope: Envelope = serde_json::from_value(json!({"outcome": "ok"})).unwrap();
    assert_eq!(envelope, Envelope::ok(json!(null)));
}

#[test]
fn list_limits_default_to_ten_and_cap_at_fifty() {
    assert_eq!(clamp_limit(None), DEFAULT_LIMIT);
    assert_eq!(clamp_limit(Some(0)), 1);
    assert_eq!(clamp_limit(Some(7)), 7);
    assert_eq!(clamp_limit(Some(500)), MAX_LIMIT);
    assert_eq!(clamp_limit(Some(u64::MAX)), MAX_LIMIT);
}
