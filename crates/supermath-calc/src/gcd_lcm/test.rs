//! Unit tests for `calc_gcd_lcm`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Project, Tool};

use super::CalcGcdLcm;

async fn call(args: Value) -> Envelope {
    let project = Project::new("t", "ws:t").unwrap();
    CalcGcdLcm.call(&CallContext::new(project, "c"), args).await
}

#[tokio::test]
async fn answers_gcd_lcm_and_coprimality() {
    let envelope = call(json!({ "a": 84, "b": 36 })).await;
    assert_eq!(envelope.data["gcd"], 12);
    assert_eq!(envelope.data["lcm"], "252");
    assert_eq!(envelope.data["coprime"], false);
    assert_eq!(call(json!({ "a": 17, "b": 5 })).await.data["coprime"], true);
}

#[tokio::test]
async fn lcm_of_two_large_values_does_not_overflow() {
    let big = u64::MAX;
    let envelope = call(json!({ "a": big, "b": big - 1 })).await;
    assert_eq!(envelope.data["gcd"], 1);
    assert_eq!(
        envelope.data["lcm"],
        (u128::from(big) * u128::from(big - 1)).to_string()
    );
}

#[tokio::test]
async fn zero_arguments_follow_the_stated_conventions() {
    let envelope = call(json!({ "a": 0, "b": 0 })).await;
    assert_eq!(envelope.data["gcd"], 0);
    assert_eq!(envelope.data["lcm"], "0");
    let envelope = call(json!({ "a": 0, "b": 9 })).await;
    assert_eq!(envelope.data["gcd"], 9);
    assert_eq!(envelope.data["lcm"], "0");
}

#[tokio::test]
async fn accepts_decimal_strings() {
    let envelope = call(json!({ "a": "18446744073709551615", "b": "5" })).await;
    assert_eq!(envelope.data["gcd"], 5);
}

#[tokio::test]
async fn refuses_missing_negative_fractional_and_oversized_arguments() {
    for args in [
        json!({ "a": 3 }),
        json!({ "b": 3 }),
        json!({ "a": -1, "b": 3 }),
        json!({ "a": 3, "b": 1.5 }),
        json!({ "a": "x", "b": 3 }),
        json!({ "a": 3, "b": "18446744073709551616" }),
        serde_json::from_str(r#"{ "a": 99999999999999999999999, "b": 3 }"#).unwrap(),
        serde_json::from_str(r#"{ "a": 3, "b": 18446744073709551616 }"#).unwrap(),
    ] {
        assert_eq!(
            call(args.clone()).await.code(),
            Some(ErrorCode::InvalidInput),
            "{args}"
        );
    }
}

#[test]
fn describes_itself_as_a_deferred_read_primitive() {
    assert_eq!(CalcGcdLcm.exposure(), Exposure::Deferred);
    assert_eq!(CalcGcdLcm.permission(), Permission::Read);
    assert_eq!(CalcGcdLcm.kind(), Kind::Primitive);
}

#[tokio::test]
async fn keeps_the_tool_contract() {
    let fixtures = [
        Fixture::new(json!({ "a": 84, "b": 36 })),
        Fixture::new(json!({ "a": -1, "b": 36 })),
    ];
    contract::check(&CalcGcdLcm, "calc", &fixtures)
        .await
        .unwrap();
}
