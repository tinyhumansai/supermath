//! `lean_search` in its three modes: the Lean service by name, and the hosted
//! Loogle and `LeanSearch` indexes, against a loopback stub.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use serde_json::{Value, json};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{ErrorCode, Outcome};
use support::{Env, Reply, Stub, config, ctx, dead_url, service};

fn names(data: &Value) -> Vec<String> {
    data["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn name_mode_asks_the_service_and_names_the_mathlib_revision() {
    let stub = service().await;
    let env = Env::over(&stub);
    let envelope = env
        .call(
            "lean_search",
            json!({"query": "Prime.two_le", "mode": "name"}),
        )
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(
        names(&envelope.data),
        ["Nat.Prime.two_le", "Nat.Prime.one_lt", "Nat.Prime.pos"]
    );
    assert_eq!(
        envelope.data["results"][0]["module"],
        "Mathlib.Data.Nat.Prime.Defs"
    );
    let coverage = envelope.coverage.unwrap();
    assert!(coverage.searched[0].contains("4f2c9e1ab07d"));
    assert!(envelope.unknowns.is_empty());
    assert_eq!(
        stub.last_body("/search"),
        json!({"query": "Prime.two_le", "limit": 10, "timeout_ms": 200})
    );
}

#[tokio::test]
async fn limits_are_clamped_and_types_are_trimmed() {
    let long = "T".repeat(500);
    let stub = Stub::start(move |_| {
        Reply::json(&json!({"status": "ok", "mathlib": "m", "results": [
            {"name": "a", "type": long, "module": "M"},
            {"name": "b", "type": "t", "module": "M"}]}))
    })
    .await;
    let env = Env::over(&stub);
    let envelope = env
        .call(
            "lean_search",
            json!({"query": "a", "mode": "name", "limit": 1}),
        )
        .await;
    assert_eq!(names(&envelope.data), ["a"]);
    assert_eq!(
        envelope.data["results"][0]["type"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        301
    );
    let sent = stub.last_body("/search");
    assert_eq!(sent["limit"], 1);
}

#[tokio::test]
async fn pattern_mode_asks_loogle_and_notes_its_other_revision() {
    let stub = service().await;
    let env = Env::over(&stub);
    let envelope = env
        .call(
            "lean_search",
            json!({"query": "(?a + ?b) ^ 2", "mode": "pattern"}),
        )
        .await;
    assert_eq!(names(&envelope.data), ["add_sq", "add_pow_two"]);
    assert_eq!(envelope.coverage.unwrap().searched, ["loogle"]);
    assert!(envelope.unknowns[0].contains("loogle indexes its own Mathlib revision"));
    let seen = stub.requests();
    assert_eq!(seen[0].method, "GET");
    assert!(seen[0].query.starts_with("q="));
    assert_eq!(stub.count("/search"), 0);
}

#[tokio::test]
async fn natural_mode_asks_leansearch() {
    let stub = service().await;
    let env = Env::over(&stub);
    let envelope = env
        .call(
            "lean_search",
            json!({"query": "sum of two squares", "mode": "natural", "limit": 5}),
        )
        .await;
    assert_eq!(
        names(&envelope.data),
        ["Nat.Prime.sq_add_sq", "Nat.eq_sq_add_sq_iff"]
    );
    assert_eq!(
        envelope.data["results"][0]["module"],
        "Mathlib.NumberTheory.SumTwoSquares"
    );
    assert_eq!(envelope.coverage.unwrap().searched, ["leansearch"]);
    assert_eq!(
        stub.last_body("/leansearch"),
        json!({"query": ["sum of two squares"], "num_results": 5})
    );
}

#[tokio::test]
async fn the_hosted_index_failures_map_to_their_codes() {
    let stub = service().await;
    let env = Env::over(&stub);
    let ask = |query: &str| json!({"query": query, "mode": "pattern"});
    let rejected = env.call("lean_search", ask("broken")).await;
    assert_eq!(rejected.code(), Some(ErrorCode::InvalidInput));
    let limited = env.call("lean_search", ask("429")).await;
    assert_eq!(limited.code(), Some(ErrorCode::RateLimited));
    assert_eq!(limited.error.unwrap().retry_after_ms, Some(7_000));
    let slow = env.call("lean_search", ask("slowq")).await;
    assert_eq!(slow.code(), Some(ErrorCode::Timeout));
    let garbage = env.call("lean_search", ask("garbage")).await;
    assert_eq!(garbage.code(), Some(ErrorCode::BackendUnavailable));
    let down = env.call("lean_search", ask("down")).await;
    assert_eq!(down.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn an_unreachable_index_is_backend_unavailable() {
    let mut cfg = config(None);
    cfg.loogle_url = format!("{}/loogle", dead_url().await);
    let env = Env::new(cfg);
    let envelope = env
        .call("lean_search", json!({"query": "a", "mode": "pattern"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn name_mode_needs_the_service_but_the_hosted_modes_do_not() {
    let stub = service().await;
    let mut cfg = config(None);
    cfg.loogle_url = format!("{}/loogle", stub.base);
    let env = Env::new(cfg);
    let name = env
        .call("lean_search", json!({"query": "a", "mode": "name"}))
        .await;
    assert_eq!(name.code(), Some(ErrorCode::BackendUnavailable));
    let pattern = env
        .call("lean_search", json!({"query": "a", "mode": "pattern"}))
        .await;
    assert_eq!(pattern.outcome, Outcome::Ok);
}

#[tokio::test]
async fn bad_arguments_are_invalid_input() {
    let stub = service().await;
    let env = Env::over(&stub);
    for args in [
        json!({"query": "  ", "mode": "name"}),
        json!({"query": "x".repeat(301), "mode": "name"}),
        json!({"query": "a", "mode": "vibes"}),
        json!({"query": "a"}),
        json!({"query": "a", "mode": "name", "extra": 1}),
    ] {
        let envelope = env.call("lean_search", args.clone()).await;
        assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput), "{args}");
    }
    assert!(stub.requests().is_empty());
}

#[tokio::test]
async fn lean_search_passes_the_contract_checker() {
    let stub = service().await;
    let env = Env::over(&stub);
    let ask =
        |query: &str, mode: &str| Fixture::with_ctx(ctx(), json!({"query": query, "mode": mode}));
    let fixtures = [
        ask("Prime", "name"),
        ask("(?a + ?b)", "pattern"),
        ask("squares", "natural"),
        Fixture::with_ctx(ctx(), json!({"query": ""})),
        ask("429", "pattern"),
        ask("slowq", "pattern"),
        ask("down", "pattern"),
    ];
    let tool = env.tool("lean_search");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
}
