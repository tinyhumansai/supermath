//! Unit tests for the client's wire forms and its unconfigured behaviour.
//! The HTTP behaviour is covered by `tests/client.rs` against a loopback stub.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use serde_json::json;

use super::*;

#[test]
fn check_request_wire_form() {
    let request = CheckRequest {
        project: "demo".into(),
        path: "lean/Supermath/A.lean".into(),
        targets: vec!["A.t".into()],
        uses: false,
        timeout_ms: 60_000,
    };
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        json!({"project": "demo", "path": "lean/Supermath/A.lean",
               "targets": ["A.t"], "uses": false, "timeout_ms": 60000})
    );
}

#[test]
fn other_requests_wire_forms() {
    let goals = GoalsRequest {
        project: "demo".into(),
        path: "p".into(),
        line: 3,
        column: 2,
        timeout_ms: 5,
    };
    assert_eq!(
        serde_json::to_value(&goals).unwrap(),
        json!({"project": "demo", "path": "p", "line": 3, "column": 2, "timeout_ms": 5})
    );
    let tactics = TacticsRequest {
        project: "demo".into(),
        path: "p".into(),
        line: 3,
        column: 2,
        tactics: vec!["simp".into()],
        timeout_ms: 5,
    };
    assert_eq!(
        serde_json::to_value(&tactics).unwrap(),
        json!({"project": "demo", "path": "p", "line": 3, "column": 2,
               "tactics": ["simp"], "timeout_ms": 5})
    );
    let decl = DeclRequest {
        project: "demo".into(),
        name: "Nat.add".into(),
        path: None,
        timeout_ms: 5,
    };
    assert_eq!(
        serde_json::to_value(&decl).unwrap(),
        json!({"project": "demo", "name": "Nat.add", "path": null, "timeout_ms": 5})
    );
    let search = SearchRequest {
        query: "prime".into(),
        limit: 10,
        timeout_ms: 5,
    };
    assert_eq!(
        serde_json::to_value(&search).unwrap(),
        json!({"query": "prime", "limit": 10, "timeout_ms": 5})
    );
    let counter = CounterRequest {
        project: "demo".into(),
        path: "p".into(),
        predicate: "P".into(),
        from: -3,
        to: 9,
        timeout_ms: 5,
    };
    assert_eq!(
        serde_json::to_value(&counter).unwrap(),
        json!({"project": "demo", "path": "p", "predicate": "P",
               "from": -3, "to": 9, "timeout_ms": 5})
    );
}

#[test]
fn replies_decode_with_missing_fields_and_unknown_statuses() {
    let reply: CheckReply = serde_json::from_value(json!({"status": "memory"})).unwrap();
    assert_eq!(reply.status, Status::Memory);
    assert!(reply.messages.is_empty());
    let odd: GoalsReply = serde_json::from_value(json!({"status": "surprise"})).unwrap();
    assert_eq!(odd.status, Status::Other);
    let row: SearchRow = serde_json::from_value(json!({"name": "a", "type": "T"})).unwrap();
    assert_eq!(row.ty, "T");
}

#[tokio::test]
async fn an_unconfigured_client_is_unavailable_everywhere() {
    let client = LeanClient::new(&LeanConfig::default()).unwrap();
    assert!(!client.is_configured());
    let health = client.health().await.unwrap_err();
    assert!(matches!(health, Error::Unavailable { .. }));
    let search = client
        .search(&SearchRequest {
            query: "x".into(),
            limit: 1,
            timeout_ms: 1,
        })
        .await
        .unwrap_err();
    assert!(matches!(search, Error::Unavailable { .. }));
}
