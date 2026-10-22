//! Unit tests for `calc_factor`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Project, Tool};

use super::CalcFactor;
use crate::FACTOR_LIMIT;

async fn call(args: Value) -> Envelope {
    let project = Project::new("t", "ws:t").unwrap();
    CalcFactor.call(&CallContext::new(project, "c"), args).await
}

#[tokio::test]
async fn reports_factors_primality_and_divisor_count() {
    let envelope = call(json!({ "n": 360 })).await;
    assert!(envelope.is_ok());
    assert_eq!(envelope.data["factorization"], "2^3 * 3^2 * 5");
    assert_eq!(envelope.data["is_prime"], false);
    assert_eq!(envelope.data["divisor_count"], 24);
    assert_eq!(
        envelope.data["factors"][0],
        json!({"prime": 2, "exponent": 3})
    );

    let envelope = call(json!({ "n": 1_000_000_007_u64 })).await;
    assert_eq!(envelope.data["is_prime"], true);
}

#[tokio::test]
async fn accepts_a_decimal_string() {
    let envelope = call(json!({ "n": "360" })).await;
    assert_eq!(envelope.data["factorization"], "2^3 * 3^2 * 5");
}

#[tokio::test]
async fn refuses_values_below_two_and_non_integers() {
    for args in [
        json!({ "n": 1 }),
        json!({ "n": 0 }),
        json!({ "n": -4 }),
        json!({ "n": 2.5 }),
        json!({ "n": "abc" }),
        json!({}),
    ] {
        assert_eq!(
            call(args.clone()).await.code(),
            Some(ErrorCode::InvalidInput),
            "{args}"
        );
    }
}

#[tokio::test]
async fn refuses_above_the_limit_without_a_partial_answer() {
    for args in [
        json!({ "n": FACTOR_LIMIT + 1 }),
        json!({ "n": "99999999999999999999999" }),
        // A bare JSON number past u64 arrives as an f64; it is too large,
        // not malformed.
        serde_json::from_str(r#"{ "n": 99999999999999999999999 }"#).unwrap(),
    ] {
        let envelope = call(args.clone()).await;
        assert_eq!(envelope.code(), Some(ErrorCode::BudgetExceeded), "{args}");
        assert!(envelope.next[0].starts_with("file_write "));
        assert!(envelope.data.is_null());
    }
    assert!(call(json!({ "n": FACTOR_LIMIT })).await.is_ok());
}

#[test]
fn describes_itself_as_a_deferred_read_primitive() {
    assert_eq!(CalcFactor.exposure(), Exposure::Deferred);
    assert_eq!(CalcFactor.permission(), Permission::Read);
    assert_eq!(CalcFactor.kind(), Kind::Primitive);
}

#[tokio::test]
async fn keeps_the_tool_contract() {
    let fixtures = [
        Fixture::new(json!({ "n": 360 })),
        Fixture::new(json!({ "n": 1 })),
        Fixture::new(json!({ "n": FACTOR_LIMIT + 1 })),
    ];
    contract::check(&CalcFactor, "calc", &fixtures)
        .await
        .unwrap();
}

/// Splits a `next` entry into its tool name and its JSON arguments.
fn call_parts(entry: &str) -> (&str, Value) {
    let (tool, arguments) = entry
        .split_once(' ')
        .unwrap_or_else(|| panic!("{entry:?} has no arguments"));
    let arguments = serde_json::from_str(arguments)
        .unwrap_or_else(|e| panic!("{entry:?} arguments are not JSON: {e}"));
    (tool, arguments)
}

#[tokio::test]
async fn a_refusal_suggests_concrete_calls_that_factor_n_in_python() {
    let envelope = call(json!({ "n": "99999999999999999999999" })).await;
    let calls: Vec<_> = envelope.next.iter().map(|e| call_parts(e)).collect();
    let tools: Vec<&str> = calls.iter().map(|(tool, _)| *tool).collect();
    assert_eq!(tools, ["file_write", "py_run"]);

    let (_, write) = &calls[0];
    let path = write["path"].as_str().unwrap();
    assert!(path.starts_with("python/work/"), "{path}");
    let source = write["content"].as_str().unwrap();
    assert!(source.contains("from sympy import factorint"), "{source}");
    assert!(source.contains("def factor(n):"), "{source}");

    let (_, run) = &calls[1];
    assert_eq!(run["path"], path);
    assert_eq!(run["entry"], "factor");
    // The exact digits, as a string, so nothing is rounded on the way.
    assert_eq!(run["args"], json!({ "n": "99999999999999999999999" }));

    let limit = call(json!({ "n": FACTOR_LIMIT + 1 })).await;
    let (_, run) = call_parts(&limit.next[1]);
    assert_eq!(run["args"]["n"], (FACTOR_LIMIT + 1).to_string());
}

#[tokio::test]
async fn a_bare_number_past_u64_gets_no_run_with_rounded_digits() {
    // An f64 has lost the digits, so only the file is suggested; the message
    // says to send `n` as a string.
    let args = serde_json::from_str(r#"{ "n": 99999999999999999999999 }"#).unwrap();
    let envelope = call(args).await;
    let tools: Vec<&str> = envelope.next.iter().map(|e| call_parts(e).0).collect();
    assert_eq!(tools, ["file_write"]);
    assert!(envelope.error.unwrap().message.contains("decimal string"));
}
