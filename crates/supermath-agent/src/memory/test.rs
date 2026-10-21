//! Unit tests for the `CortexDB` client, against a loopback stub server.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::{Memory, idempotency_key, pack, reached_barrier, render_pack};
use crate::error::Error;
use crate::problem::Problem;

/// One request the stub received.
#[derive(Debug)]
struct Seen {
    head: String,
    body: Value,
}

/// Serves exactly one HTTP request with `status` and `body`, then hands back
/// what it received.
async fn stub(status: u16, body: &str) -> (String, tokio::task::JoinHandle<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_string();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];
        let (head, length) = loop {
            let read = socket.read(&mut chunk).await.unwrap();
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&buffer[..end]).to_string();
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                buffer.drain(..end + 4);
                break (head, length);
            }
        };
        while buffer.len() < length {
            let read = socket.read(&mut chunk).await.unwrap();
            buffer.extend_from_slice(&chunk[..read]);
        }
        let response = format!(
            "HTTP/1.1 {status} STUB\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        Seen {
            head,
            body: serde_json::from_slice(&buffer).unwrap_or(Value::Null),
        }
    });
    (url, handle)
}

fn problem() -> Problem {
    Problem::new(
        "odd-sums",
        "Show the sum of the first n odd numbers is n^2.",
    )
    .unwrap()
}

#[test]
fn scope_is_built_from_the_root_and_the_slug_only() {
    let memory = Memory::new("http://x/", "k", "ws:supermath");
    assert_eq!(
        memory.scope(&problem()),
        "ws:supermath/project:odd-sums/app:brain"
    );
}

#[tokio::test]
async fn recall_posts_the_scope_and_renders_the_pack() {
    let pack = json!({ "layers": { "events": [{ "content": { "text": "n=1..5 checked" } }] } });
    let (url, seen) = stub(200, &pack.to_string()).await;
    let memory = Memory::new(format!("{url}/"), "secret", "ws:t");
    let recalled = memory.recall(&problem(), "odd sums").await.unwrap();
    assert_eq!(
        recalled.as_deref(),
        Some("### Notes recorded\n\n1. n=1..5 checked")
    );
    let seen = seen.await.unwrap();
    assert!(seen.head.starts_with("POST /v1/recall "), "{}", seen.head);
    assert!(
        seen.head
            .to_ascii_lowercase()
            .contains("authorization: bearer secret")
    );
    assert_eq!(seen.body["scope"], "ws:t/project:odd-sums/app:brain");
    assert_eq!(seen.body["query"], "odd sums");
    assert_eq!(seen.body["budgets"]["per_layer_limits"]["event"], 8);
}

#[tokio::test]
async fn recall_of_an_unknown_scope_is_nothing_recorded() {
    let (url, _seen) = stub(404, r#"{"error_code":"NOT_FOUND"}"#).await;
    let memory = Memory::new(url, "k", "ws:t");
    assert_eq!(memory.recall(&problem(), "q").await.unwrap(), None);
}

#[tokio::test]
async fn a_refusal_names_the_server_error_code() {
    let (url, _seen) = stub(403, r#"{"error_code":"POLICY_DENIED","message":"no"}"#).await;
    let memory = Memory::new(url, "k", "ws:t");
    let error = memory.recall(&problem(), "q").await.unwrap_err();
    assert!(
        matches!(&error, Error::MemoryRefused { operation: "recall", status: 403, detail } if detail == "POLICY_DENIED: no"),
        "{error}"
    );
}

#[tokio::test]
async fn invalid_json_is_a_transport_error() {
    let (url, _seen) = stub(200, "not json").await;
    let memory = Memory::new(url, "k", "ws:t");
    let error = memory.recall(&problem(), "q").await.unwrap_err();
    assert!(matches!(
        error,
        Error::MemoryTransport {
            operation: "recall",
            ..
        }
    ));
}

#[tokio::test]
async fn an_unreachable_server_is_a_transport_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let memory = Memory::new(url, "k", "ws:t");
    let error = memory.remember(&problem(), "note", &[]).await.unwrap_err();
    assert!(matches!(
        error,
        Error::MemoryTransport {
            operation: "remember",
            ..
        }
    ));
}

#[tokio::test]
async fn remember_waits_for_the_index_and_returns_the_event_id() {
    let answer = json!({ "event_id": "ev-1", "stages_completed": ["stored", "indexed"] });
    let (url, seen) = stub(200, &answer.to_string()).await;
    let memory = Memory::new(url, "k", "ws:t");
    let id = memory
        .remember(&problem(), "VERDICT: proved", &["verdict:proved"])
        .await
        .unwrap();
    assert_eq!(id, "ev-1");
    let seen = seen.await.unwrap();
    assert!(seen.head.starts_with("POST /v1/experience?wait=indexed "));
    assert_eq!(seen.body["content"]["text"], "VERDICT: proved");
    assert_eq!(seen.body["context"]["labels"][0], "verdict:proved");
    assert_eq!(seen.body["idempotency_key"].as_str().unwrap().len(), 64);
}

#[tokio::test]
async fn remember_refuses_a_write_that_was_not_indexed() {
    let answer = json!({ "event_id": "ev-1", "stages_completed": ["stored"] });
    let (url, _seen) = stub(200, &answer.to_string()).await;
    let memory = Memory::new(url, "k", "ws:t");
    let error = memory.remember(&problem(), "n", &[]).await.unwrap_err();
    assert!(matches!(error, Error::MemoryNotIndexed { completed } if completed == "stored"));
}

#[test]
fn barrier_accepts_a_missing_stage_list_and_names_an_empty_one() {
    assert!(reached_barrier(&json!({}), "indexed").is_ok());
    let error = reached_barrier(&json!({ "stages_completed": [] }), "indexed").unwrap_err();
    assert!(matches!(error, Error::MemoryNotIndexed { completed } if completed == "nothing"));
}

#[test]
fn render_pack_skips_empty_layers_and_reads_each_layer_shape() {
    let pack = json!({ "layers": {
        "events": [],
        "facts": [{ "statement": "1+3=4" }],
        "beliefs": [{ "summary": "holds for n<=5" }, { "id": 7 }],
    }});
    let rendered = render_pack(&pack, &["events", "facts", "beliefs"]).unwrap();
    assert_eq!(
        rendered,
        "### Facts extracted\n\n1. 1+3=4\n\n### What memory currently holds\n\n1. holds for n<=5\n2. {\"id\":7}"
    );
    assert_eq!(render_pack(&json!({}), &["events"]), None);
}

#[test]
fn render_pack_clips_a_long_passage() {
    let long = "x".repeat(2_000);
    let pack = json!({ "layers": { "events": [{ "content": { "text": long } }] } });
    let rendered = render_pack(&pack, &["events"]).unwrap();
    assert!(rendered.ends_with('…'));
    assert!(rendered.chars().count() < 1_300);
}

#[test]
fn refusal_detail_falls_back_to_the_body() {
    assert_eq!(
        pack::refusal_detail(r#"{"error_code":"RATE_LIMITED"}"#),
        "RATE_LIMITED"
    );
    assert_eq!(
        pack::refusal_detail("  gateway timeout "),
        "gateway timeout"
    );
}

#[test]
fn singular_spells_budget_keys_the_server_reads() {
    assert_eq!(pack::singular("events"), "event");
    assert_eq!(pack::singular("episodes"), "episode");
    assert_eq!(pack::singular("facts"), "fact");
    assert_eq!(pack::singular("beliefs"), "belief");
    assert_eq!(pack::singular("understanding"), "understanding");
}

#[test]
fn idempotency_keys_are_stable_and_content_addressed() {
    let a = idempotency_key("s", "note");
    assert_eq!(a, idempotency_key("s", "note"));
    assert_ne!(a, idempotency_key("s", "note2"));
    assert_ne!(a, idempotency_key("s2", "note"));
    assert_eq!(a.len(), 64);
}
