//! Tests for `web_search` argument handling and prompt building.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::Request;
use crate::source::SourceClass;

#[test]
fn parses_a_full_request_and_clamps_the_limit() {
    let request = Request::parse(json!({
        "question": "  best sunflower bound?  ",
        "prefer": ["preprint", "qa"],
        "since": "2019-01-01",
        "limit": 500
    }))
    .unwrap();
    assert_eq!(request.question, "best sunflower bound?");
    assert_eq!(request.prefer, [SourceClass::Preprint, SourceClass::Qa]);
    assert_eq!(request.limit, 20);
    let prompt = request.prompt();
    assert!(prompt.contains("MathOverflow"));
    assert!(prompt.contains("2019-01-01"));
    assert!(prompt.ends_with("Question: best sunflower bound?"));
}

#[test]
fn the_default_limit_is_eight_and_zero_becomes_one() {
    assert_eq!(Request::parse(json!({"question": "q"})).unwrap().limit, 8);
    assert_eq!(
        Request::parse(json!({"question": "q", "limit": 0}))
            .unwrap()
            .limit,
        1
    );
}

#[test]
fn refuses_bad_arguments() {
    let long = "x".repeat(501);
    for args in [
        json!({}),
        json!({"question": "   "}),
        json!({"question": long}),
        json!({"question": "q", "prefer": ["podcast"]}),
        json!({"question": "q", "since": "2019-02-30"}),
        json!({"question": "q", "extra": 1}),
        json!({"question": "q", "limit": -1}),
    ] {
        assert!(Request::parse(args.clone()).is_err(), "{args}");
    }
}

#[test]
fn a_plain_prompt_has_no_preference_or_date_lines() {
    let prompt = Request::parse(json!({"question": "q"})).unwrap().prompt();
    assert!(!prompt.contains("Prefer"));
    assert!(!prompt.contains("onwards"));
    assert!(prompt.contains("No source found"));
}
