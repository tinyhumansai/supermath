//! The public contract: both tools through a `ToolRegistry`, as a host uses them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use supermath_calc::{CalcFactor, CalcGcdLcm};
use supermath_tool::{CallContext, ErrorCode, Project, Tool};

fn ctx() -> CallContext {
    CallContext::new(Project::new("odd-sums", "ws:t").unwrap(), "call-1")
}

#[tokio::test]
async fn both_tools_answer_through_the_public_api() {
    let ctx = ctx();
    let factor = CalcFactor
        .call(&ctx, json!({ "n": 1_000_000_007_u64 }))
        .await;
    assert_eq!(factor.data["is_prime"], true);
    let gcd = CalcGcdLcm.call(&ctx, json!({ "a": 84, "b": 36 })).await;
    assert_eq!(gcd.data["lcm"], "252");
}

#[tokio::test]
async fn both_tools_refuse_with_stable_codes() {
    let ctx = ctx();
    let factor = CalcFactor.call(&ctx, json!({ "n": 1 })).await;
    assert_eq!(factor.code(), Some(ErrorCode::InvalidInput));
    let gcd = CalcGcdLcm.call(&ctx, json!({ "a": -2, "b": 1 })).await;
    assert_eq!(gcd.code(), Some(ErrorCode::InvalidInput));
}
