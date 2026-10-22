//! Unit tests for the `CortexDB` client, against a loopback stub server.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use tokio::net::TcpListener;

use super::stub::{FakeCortex, stub};
use super::{
    Memory, Project, content_digest, idempotency_key, pack, reached_barrier, render_pack, slugify,
};
use crate::error::Error;

fn project() -> Project {
    Project::new("odd-sums").unwrap()
}

#[test]
fn scope_is_built_from_the_root_and_the_slug_only() {
    let memory = Memory::new("http://x/", "k", "ws:supermath");
    assert_eq!(
        memory.scope(&project()),
        "ws:supermath/project:odd-sums/app:brain"
    );
}

#[tokio::test]
async fn recall_posts_the_scope_and_renders_the_pack() {
    let pack = json!({ "layers": { "events": [{ "content": { "text": "n=1..5 checked" } }] } });
    let (url, seen) = stub(200, &pack.to_string()).await;
    let memory = Memory::new(format!("{url}/"), "secret", "ws:t");
    let recalled = memory.recall(&project(), "odd sums").await.unwrap();
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
    assert_eq!(memory.recall(&project(), "q").await.unwrap(), None);
}

#[tokio::test]
async fn a_refusal_names_the_server_error_code() {
    let (url, _seen) = stub(403, r#"{"error_code":"POLICY_DENIED","message":"no"}"#).await;
    let memory = Memory::new(url, "k", "ws:t");
    let error = memory.recall(&project(), "q").await.unwrap_err();
    assert!(
        matches!(&error, Error::MemoryRefused { operation: "recall", status: 403, detail } if detail == "POLICY_DENIED: no"),
        "{error}"
    );
}

#[tokio::test]
async fn invalid_json_is_a_transport_error() {
    let (url, _seen) = stub(200, "not json").await;
    let memory = Memory::new(url, "k", "ws:t");
    let error = memory.recall(&project(), "q").await.unwrap_err();
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
    let error = memory.remember(&project(), "note", &[]).await.unwrap_err();
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
        .remember(&project(), "VERDICT: proved", &["verdict:proved"])
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
    let error = memory.remember(&project(), "n", &[]).await.unwrap_err();
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

#[test]
fn slugify_keeps_lowercase_letters_digits_and_single_dashes() {
    assert_eq!(
        slugify("Sum of Odd Numbers").as_deref(),
        Some("sum-of-odd-numbers")
    );
    assert_eq!(
        slugify("erdos_gyarfas--2").as_deref(),
        Some("erdos-gyarfas-2")
    );
    assert_eq!(slugify("  ../Collatz!  ").as_deref(), Some("collatz"));
    assert_eq!(slugify("--//--"), None);
    assert_eq!(slugify("∑∏"), None);
}

#[test]
fn project_names_are_normalised_or_refused() {
    let project = Project::new("Twin Primes").unwrap();
    assert_eq!(project.slug(), "twin-primes");
    assert_eq!(project.to_string(), "twin-primes");
    let error = Project::new("!!").unwrap_err();
    assert!(matches!(error, Error::InvalidProject(name) if name == "!!"));
}

#[tokio::test]
async fn ready_accepts_a_ready_server() {
    let (url, seen) = stub(200, r#"{"ready":true,"degraded":false}"#).await;
    Memory::new(url, "k", "ws:t").ready().await.unwrap();
    assert!(seen.await.unwrap().head.starts_with("GET /v1/admin/ready "));
}

#[tokio::test]
async fn ready_names_the_failing_checks_of_an_unready_server() {
    let body = r#"{"ready":false,"checks":{"embeddings":{"ok":false,"detail":"no key"},"wal":{"ok":true}}}"#;
    let (url, _seen) = stub(503, body).await;
    let error = Memory::new(url, "k", "ws:t").ready().await.unwrap_err();
    assert!(
        matches!(&error, Error::MemoryNotReady(detail) if detail == "embeddings: no key"),
        "{error}"
    );
}

#[tokio::test]
async fn ready_refuses_a_server_on_mock_embeddings() {
    let (url, _seen) = stub(
        200,
        r#"{"ready":true,"degraded":true,"embedding_provider":"mock"}"#,
    )
    .await;
    let error = Memory::new(url, "k", "ws:t").ready().await.unwrap_err();
    assert!(
        matches!(&error, Error::MemoryNotReady(detail) if detail.contains("degraded")),
        "{error}"
    );
}

#[tokio::test]
async fn ready_reports_an_unreachable_server_and_a_garbled_answer() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let error = Memory::new(url, "k", "ws:t").ready().await.unwrap_err();
    assert!(matches!(
        error,
        Error::MemoryTransport {
            operation: "ready",
            ..
        }
    ));

    let (url, _seen) = stub(200, "not json").await;
    let error = Memory::new(url, "k", "ws:t").ready().await.unwrap_err();
    assert!(matches!(
        error,
        Error::MemoryTransport {
            operation: "ready",
            ..
        }
    ));
}

#[tokio::test]
async fn recall_notes_returns_event_texts_and_bounds_the_window() {
    let pack = json!({ "layers": { "events": [
        { "content": { "text": "first" } },
        { "id": 7 },
        { "content": { "text": "second" } },
    ] } });
    let (url, seen) = stub(200, &pack.to_string()).await;
    let memory = Memory::new(url, "k", "ws:t");
    let notes = memory.recall_notes(&project(), "q", 40).await.unwrap();
    assert_eq!(notes, ["first", "second"]);
    let seen = seen.await.unwrap();
    assert_eq!(seen.body["include"], json!(["events"]));
    assert_eq!(seen.body["budgets"]["per_layer_limits"]["event"], 40);
}

#[tokio::test]
async fn recall_notes_of_an_unknown_scope_is_empty() {
    let (url, _seen) = stub(404, "{}").await;
    let memory = Memory::new(url, "k", "ws:t");
    assert_eq!(
        memory.recall_notes(&project(), "q", 5).await.unwrap().len(),
        0
    );
}

#[test]
fn content_digests_are_sixteen_hex_digits_and_separate_their_parts() {
    let a = content_digest(&["result", "claim", "{}"]);
    assert_eq!(a.len(), 16);
    assert_eq!(a, content_digest(&["result", "claim", "{}"]));
    assert_ne!(a, content_digest(&["result", "claim", "{ }"]));
    assert_ne!(content_digest(&["ab", "c"]), content_digest(&["a", "bc"]));
}

#[tokio::test]
async fn the_fake_cortex_stores_recalls_newest_first_and_dedupes_by_key() {
    let cortex = FakeCortex::start().await;
    let memory = Memory::new(cortex.url(), "k", "ws:t");
    let one = Project::new("one").unwrap();
    let other = Project::new("other").unwrap();
    assert_eq!(memory.recall_notes(&one, "q", 10).await.unwrap().len(), 0);
    let first = memory.remember(&one, "alpha", &["kind:x"]).await.unwrap();
    memory.remember(&one, "beta", &[]).await.unwrap();
    memory.remember(&other, "gamma", &[]).await.unwrap();
    assert_eq!(
        memory.remember(&one, "alpha", &["kind:x"]).await.unwrap(),
        first
    );
    assert_eq!(cortex.events().len(), 3);
    assert_eq!(cortex.events()[0].labels, ["kind:x"]);
    assert_eq!(
        memory.recall_notes(&one, "q", 10).await.unwrap(),
        ["beta", "alpha"]
    );
    assert_eq!(memory.recall_notes(&one, "q", 1).await.unwrap(), ["beta"]);
    assert_eq!(cortex.recalls(), 3);
}

#[tokio::test]
async fn the_fake_cortex_can_refuse_or_skip_the_index() {
    let cortex = FakeCortex::start().await;
    let memory = Memory::new(cortex.url(), "k", "ws:t");
    cortex.refuse_with(Some(503));
    let error = memory.remember(&project(), "n", &[]).await.unwrap_err();
    assert!(matches!(error, Error::MemoryRefused { status: 503, .. }));
    cortex.refuse_with(None);
    cortex.skip_index(true);
    let error = memory.remember(&project(), "n", &[]).await.unwrap_err();
    assert!(matches!(error, Error::MemoryNotIndexed { .. }));
    cortex.skip_index(false);
    cortex.refuse_scope(Some("project:odd-sums"));
    assert!(memory.recall_notes(&project(), "q", 1).await.is_err());
    assert!(
        memory
            .recall_notes(&Project::new("fine").unwrap(), "q", 1)
            .await
            .is_ok()
    );
    let (url, _seen) = stub(200, "{}").await;
    let response = reqwest::get(format!("{url}/v1/unknown")).await.unwrap();
    assert_eq!(response.status(), 200);
    let response = reqwest::get(format!("{}/v1/unknown", cortex.url()))
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn notes_labelled_lists_exactly_the_labelled_notes_across_pages() {
    let cortex = FakeCortex::start().await;
    let memory = Memory::new(cortex.url(), "k", "ws:t");
    // More than one page of the label, and notes without it in between.
    for n in 0..105 {
        memory
            .remember(&project(), &format!("note {n}"), &["claim:clm_a"])
            .await
            .unwrap();
        if n % 50 == 0 {
            memory
                .remember(&project(), &format!("other {n}"), &["claim:clm_b"])
                .await
                .unwrap();
        }
    }
    let notes = memory
        .notes_labelled(&project(), "claim:clm_a")
        .await
        .unwrap();
    assert_eq!(notes.len(), 105);
    assert_eq!(notes[0], "note 0");
    assert_eq!(notes[104], "note 104");
    assert!(
        memory
            .notes_labelled(&project(), "claim:clm_none")
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn notes_labelled_sends_the_scope_and_label_and_reads_an_unknown_scope_as_empty() {
    let (url, seen) = stub(404, r#"{"error_code":"NOT_FOUND"}"#).await;
    let memory = Memory::new(url, "secret", "ws:t");
    let notes = memory
        .notes_labelled(&project(), "claim:clm_a")
        .await
        .unwrap();
    assert!(notes.is_empty());
    let head = seen.await.unwrap().head;
    assert!(head.starts_with("GET /v1/events?"), "{head}");
    assert!(head.contains("labels=claim%3Aclm_a"), "{head}");
    assert!(
        head.contains("scope=ws%3At%2Fproject%3Aodd-sums%2Fapp%3Abrain"),
        "{head}"
    );

    let (url, _seen) = stub(403, r#"{"error_code":"POLICY_DENIED","message":"no"}"#).await;
    let memory = Memory::new(url, "k", "ws:t");
    let error = memory
        .notes_labelled(&project(), "claim:clm_a")
        .await
        .unwrap_err();
    assert!(matches!(error, Error::MemoryRefused { .. }), "{error}");
}
