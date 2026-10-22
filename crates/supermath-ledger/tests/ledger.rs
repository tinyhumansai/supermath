//! The ledger end to end through its public surface: a configuration, the
//! composed tools in a registry, a loopback `CortexDB`, and a scripted Lean.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// The stub is compiled straight from the library's source so this test runs
// without the `test-support` feature.
#![allow(unreachable_pub, missing_docs, dead_code, clippy::must_use_candidate)]

#[path = "../src/memory/stub.rs"]
mod stub;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use supermath_ledger::{Config, LeanEvidence, LeanVerdict, LeanVerdictKind, Ledger, Result, tools};
use supermath_tool::{
    CallContext, CallLog, CallRecord, Envelope, ErrorCode, InMemoryCallLog, Outcome, Project,
    ToolRegistry,
};

/// A Lean that checks `Main.thm` cleanly at whatever digest it is told.
struct CleanLean {
    sha: std::sync::Mutex<String>,
}

#[async_trait]
impl LeanEvidence for CleanLean {
    async fn verify(&self, _path: &str, declaration: &str) -> Result<LeanVerdict> {
        Ok(LeanVerdict {
            verdict: LeanVerdictKind::Checked,
            file_sha256: self.sha.lock().unwrap().clone(),
            sorries: 0,
            debt: vec![],
            axioms: [(declaration.to_string(), vec!["propext".to_string()])].into(),
            provenance: Value::Null,
        })
    }
}

fn ctx() -> CallContext {
    CallContext::new(Project::new("twin-primes", "ws:t").unwrap(), "call-1")
}

#[tokio::test]
async fn a_project_records_links_and_reads_claims_through_the_registry() {
    let cortex = stub::FakeCortex::start().await;
    let config = Config::from_lookup(|name| match name {
        "CORTEX_API_KEY" => Some("key".to_string()),
        "CORTEX_API_URL" => Some(cortex.url().to_string()),
        "CORTEX_SCOPE_ROOT" => Some("ws:it".to_string()),
        _ => None,
    })
    .unwrap();
    let memory = config.memory();
    let lean = Arc::new(CleanLean {
        sha: std::sync::Mutex::new("aa".into()),
    });
    let calls = Arc::new(InMemoryCallLog::new());
    let ledger = Arc::new(Ledger::new(memory.clone(), lean.clone(), calls.clone()));
    let mut registry = ToolRegistry::new();
    for tool in tools(&memory, &ledger) {
        registry.register(tool).unwrap();
    }
    let call = |name: &'static str, args: Value| {
        let tool = Arc::clone(registry.get(name).unwrap());
        async move { tool.call(&ctx(), args).await }
    };

    let lemma = call(
        "ledger_record",
        json!({"kind": "fact", "statement": "a lemma",
        "source": {"arxiv": {"id": "1", "version": "v1", "label": "lem:1"}}}),
    )
    .await;
    assert_eq!(lemma.data["status"], "cited");

    let main = call("ledger_record", json!({"kind": "result", "statement": "the theorem",
        "source": {"lean": {"path": "lean/Main.lean", "declaration": "Main.thm", "file_sha256": "aa"}}})).await;
    assert_eq!(main.data["status"], "formalised");
    let (lemma_id, main_id) = (
        lemma.data["id"].as_str().unwrap().to_string(),
        main.data["id"].as_str().unwrap().to_string(),
    );

    let link = call("ledger_link", json!({"from": main_id, "to": lemma_id})).await;
    assert_eq!(link.outcome, Outcome::Ok);
    let cycle = call("ledger_link", json!({"from": lemma_id, "to": main_id})).await;
    assert_eq!(cycle.code(), Some(ErrorCode::InvalidInput));

    let one = call("ledger_claims", json!({"id": main_id})).await;
    assert_eq!(one.data["effective_status"], "cited");
    assert_eq!(one.data["limited_by"], json!([main_id, lemma_id]));

    // The file changes: the formalised claim stands as asserted.
    *lean.sha.lock().unwrap() = "bb".into();
    let stale = call("ledger_claims", json!({"id": main_id})).await;
    assert_eq!(stale.data["effective_status"], "asserted");

    let forged = call(
        "ledger_record",
        json!({"kind": "result", "statement": "x",
        "source": {"supplied": "y"}, "status": "formalised"}),
    )
    .await;
    assert_eq!(forged.code(), Some(ErrorCode::Forbidden));

    calls.record(CallRecord::new(
        "run-1",
        "py_run",
        "twin-primes",
        &Envelope::ok(json!({})),
        4,
    ));
    let checked = call(
        "ledger_bound",
        json!({"quantity": "gap", "direction": "upper",
        "value": 246, "frame": "primes < 10^9",
        "source": {"tool": {"tool": "py_run", "call_id": "run-1"}}}),
    )
    .await;
    assert_eq!(checked.data["recorded"]["status"], "checked");

    let recalled = call("recall_memory", json!({"query": "the theorem"})).await;
    assert!(
        recalled.data["notes"]
            .as_str()
            .unwrap()
            .contains("the theorem")
    );
    assert!(
        cortex
            .events()
            .iter()
            .all(|e| e.scope == "ws:it/project:twin-primes/app:brain"),
        "every write is in the host's project scope"
    );
}
