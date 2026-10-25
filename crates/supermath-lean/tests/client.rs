//! The HTTP client against a loopback stub: every route, and every way the
//! service can fail.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use serde_json::json;
use supermath_lean::{
    CheckRequest, Error, GoalsRequest, LeanClient, SearchRequest, Status, TacticsRequest,
};
use support::{Reply, Stub, config, dead_url, service};

fn check_request() -> CheckRequest {
    CheckRequest {
        project: "demo".into(),
        root: None,
        path: "lean/Supermath/Clean.lean".into(),
        targets: vec!["two_add_two".into()],
        uses: false,
        timeout_ms: 1_000,
    }
}

#[tokio::test]
async fn health_reports_the_pin() {
    let stub = service().await;
    let client = LeanClient::new(&config(Some(&stub.base))).unwrap();
    assert!(client.is_configured());
    let health = client.health().await.unwrap();
    assert_eq!(health.toolchain, "leanprover/lean4:v4.20.0");
    assert_eq!(health.mathlib, "4f2c9e1ab07d");
    assert_eq!(stub.requests()[0].method, "GET");
}

#[tokio::test]
async fn check_decodes_the_recorded_answer() {
    let stub = service().await;
    let client = LeanClient::new(&config(Some(&stub.base))).unwrap();
    let reply = client.check(&check_request()).await.unwrap();
    assert_eq!(reply.status, Status::Ok);
    assert_eq!(reply.elapsed_ms, 1840);
    assert_eq!(
        reply.axioms["two_add_two"],
        Some(vec!["propext".to_owned()])
    );
}

#[tokio::test]
async fn every_route_posts_json_to_its_own_path() {
    let stub = service().await;
    let client = LeanClient::new(&config(Some(&stub.base))).unwrap();
    client
        .goals(&GoalsRequest {
            project: "demo".into(),
            root: None,
            path: "lean/Supermath/Main.lean".into(),
            line: 1,
            column: 0,
            timeout_ms: 1_000,
        })
        .await
        .unwrap();
    let tactics = client
        .tactics(&TacticsRequest {
            project: "demo".into(),
            root: None,
            path: "lean/Supermath/Main.lean".into(),
            line: 1,
            column: 0,
            tactics: vec!["a".into(), "b".into(), "c".into()],
            timeout_ms: 1_000,
        })
        .await
        .unwrap();
    assert_eq!(tactics.results.len(), 3);
    client
        .search(&SearchRequest {
            query: "x".into(),
            limit: 3,
            timeout_ms: 1_000,
        })
        .await
        .unwrap();
    let routes: Vec<_> = stub.requests().iter().map(|r| r.path.clone()).collect();
    assert_eq!(routes, ["/goals", "/tactics", "/search"]);
}

#[tokio::test]
async fn status_codes_map_to_errors() {
    for (status, expect) in [
        (400, "invalid"),
        (404, "not_found"),
        (500, "unavailable"),
        (502, "unavailable"),
    ] {
        let stub = Stub::start(move |_| Reply::status(status, "because")).await;
        let client = LeanClient::new(&config(Some(&stub.base))).unwrap();
        let error = client.check(&check_request()).await.unwrap_err();
        let got = match &error {
            Error::InvalidInput(m) => {
                assert_eq!(m, "because");
                "invalid"
            }
            Error::NotFound(_) => "not_found",
            Error::Unavailable { reason, .. } => {
                assert!(reason.contains(&status.to_string()), "{reason}");
                "unavailable"
            }
            other => panic!("{other}"),
        };
        assert_eq!(got, expect);
    }
}

#[tokio::test]
async fn an_error_without_a_json_body_still_maps_by_status() {
    let stub = Stub::start(|_| Reply {
        status: 404,
        body: b"nope".to_vec(),
        delay_ms: 0,
        headers: Vec::new(),
    })
    .await;
    let client = LeanClient::new(&config(Some(&stub.base))).unwrap();
    let error = client.check(&check_request()).await.unwrap_err();
    assert!(
        matches!(&error, Error::NotFound(m) if m == "status 404"),
        "{error}"
    );
}

#[tokio::test]
async fn an_unreadable_success_body_is_unavailable() {
    let stub = Stub::start(|_| Reply::raw("<html>")).await;
    let client = LeanClient::new(&config(Some(&stub.base))).unwrap();
    let error = client.check(&check_request()).await.unwrap_err();
    assert!(matches!(error, Error::Unavailable { .. }));
}

#[tokio::test]
async fn a_slow_service_times_out_and_a_dead_one_is_unavailable() {
    let stub = Stub::start(|_| Reply::json(&json!({"status": "ok"})).delayed(400)).await;
    let client = LeanClient::new(&config(Some(&stub.base))).unwrap();
    let mut request = check_request();
    request.timeout_ms = 50;
    assert!(matches!(
        client.check(&request).await.unwrap_err(),
        Error::Timeout(_)
    ));

    let dead = LeanClient::new(&config(Some(&dead_url().await))).unwrap();
    assert!(matches!(
        dead.health().await.unwrap_err(),
        Error::Unavailable { .. }
    ));
}
