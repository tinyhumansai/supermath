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
        assert_eq!(envelope.next.len(), 2);
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
