//! The ledger tools: the tool contract over every one, the trust rules
//! through their arguments, and the shape of each answer.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use serde_json::{Value, json};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{
    Budget, CallContext, CallLog, CallRecord, Envelope, ErrorCode, InMemoryCallLog, Outcome,
    Project, Tool, ToolRegistry,
};

use super::{LedgerBound, LedgerClaims, LedgerLink, LedgerRecord, RecallMemory, tools, trim};
use crate::evidence::{FakeLeanEvidence, LeanVerdict, LeanVerdictKind};
use crate::ledger::{Claim, Ledger};
use crate::memory::Memory;
use crate::memory::stub::FakeCortex;
use crate::types::{Kind, Source};

const AXIOMS: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];

struct World {
    cortex: FakeCortex,
    lean: Arc<FakeLeanEvidence>,
    calls: Arc<InMemoryCallLog>,
    memory: Memory,
    ledger: Arc<Ledger>,
}

async fn world() -> World {
    let cortex = FakeCortex::start().await;
    let lean = Arc::new(FakeLeanEvidence::new());
    let calls = Arc::new(InMemoryCallLog::new());
    cortex.refuse_scope(Some("project:down"));
    let memory = Memory::new(cortex.url(), "k", "ws:t");
    let ledger = Arc::new(Ledger::new(memory.clone(), lean.clone(), calls.clone()));
    World {
        cortex,
        lean,
        calls,
        memory,
        ledger,
    }
}

fn ctx_for(slug: &str) -> CallContext {
    CallContext::new(Project::new(slug, "ws:t").unwrap(), "call-1")
}

fn ctx() -> CallContext {
    ctx_for("sendov")
}

fn fixture(args: Value) -> Fixture {
    Fixture::with_ctx(ctx(), args)
}

fn down(args: Value) -> Fixture {
    Fixture::with_ctx(ctx_for("down"), args)
}

fn verdict(sha: &str) -> LeanVerdict {
    FakeLeanEvidence::clean(sha, "Main.thm", &AXIOMS)
}

fn teach(w: &World, sha: &str) {
    w.lean.set("lean/Main.lean", "Main.thm", Ok(verdict(sha)));
}

fn lean_source(sha: &str) -> Value {
    json!({"lean": {"path": "lean/Main.lean", "declaration": "Main.thm", "file_sha256": sha}})
}

fn supplied_source() -> Value {
    json!({"supplied": "a remark"})
}

async fn call(tool: &dyn Tool, args: Value) -> Envelope {
    tool.call(&ctx(), args).await
}

async fn seed(w: &World, text: &str) -> String {
    w.ledger
        .record(
            &ctx(),
            Claim::new(Kind::Result, text, Source::Supplied("s".into())),
        )
        .await
        .unwrap()
        .entry
        .id
}

fn id_of(envelope: &Envelope) -> String {
    envelope.data["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn recall_memory_keeps_the_tool_contract() {
    let w = world().await;
    w.memory
        .remember(
            &crate::memory::Project::new("sendov").unwrap(),
            "n=1..5 checked",
            &[],
        )
        .await
        .unwrap();
    let tool = RecallMemory::new(w.memory.clone());
    let tight = Budget {
        bytes: 4,
        ..Budget::default()
    };
    let fixtures = [
        fixture(json!({"query": "checked"})),
        Fixture::with_ctx(ctx_for("empty"), json!({"query": "anything"})),
        fixture(json!({"query": "   "})),
        down(json!({"query": "q"})),
        Fixture::with_ctx(ctx().with_budget(tight), json!({"query": "a long query"})),
    ];
    contract::check(&tool, "ledger", &fixtures).await.unwrap();
}

#[tokio::test]
async fn recall_memory_asks_cortexdb_for_the_granular_view_of_the_hosts_scope() {
    let (url, seen) = crate::memory::stub::stub(404, r#"{"error_code":"NOT_FOUND"}"#).await;
    let tool = RecallMemory::new(Memory::new(url, "k", "ws:t"));
    let answer = call(&tool, json!({"query": "bounds"})).await;
    assert_eq!(answer.data, json!({"notes": ""}));
    let request = seen.await.unwrap();
    assert!(
        request.head.starts_with("POST /v1/recall"),
        "{}",
        request.head
    );
    assert_eq!(request.body["view"], "granular");
    assert_eq!(request.body["scope"], "ws:t/project:sendov/app:brain");
    assert_eq!(request.body["query"], "bounds");
}

#[tokio::test]
async fn recall_memory_answers_notes_or_an_honest_empty() {
    let w = world().await;
    let tool = RecallMemory::new(w.memory.clone());
    assert_eq!(tool.name(), "recall_memory");
    let empty = call(&tool, json!({"query": "q"})).await;
    assert_eq!(empty.outcome, Outcome::Ok);
    assert_eq!(empty.data["notes"], "");
    assert!(empty.unknowns[0].contains("similarity"));
    assert!(empty.coverage.is_some());

    w.memory
        .remember(
            &crate::memory::Project::new("sendov").unwrap(),
            &"x".repeat(9_000),
            &[],
        )
        .await
        .unwrap();
    let found = call(&tool, json!({"query": "q"})).await;
    let notes = found.data["notes"].as_str().unwrap();
    assert!(notes.contains("Notes recorded"));
    assert!(notes.chars().count() <= 8_001, "notes are trimmed");
    assert_eq!(found.unknowns.len(), 0);
}

#[tokio::test]
async fn recall_memory_reads_the_hosts_project_and_nothing_a_caller_names() {
    let w = world().await;
    let tool = RecallMemory::new(w.memory.clone());
    let smuggled = call(
        &tool,
        json!({"query": "q", "scope": "ws:t/project:other/app:brain"}),
    )
    .await;
    assert_eq!(smuggled.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn ledger_record_keeps_the_tool_contract() {
    let w = world().await;
    teach(&w, "bb");
    let tool = LedgerRecord::new(w.ledger.clone());
    let tight = Budget {
        bytes: 5,
        ..Budget::default()
    };
    let fixtures = [
        fixture(json!({"kind": "result", "statement": "n^2 is even", "source": supplied_source()})),
        fixture(json!({"kind": "result", "statement": "  ", "source": supplied_source()})),
        fixture(
            json!({"kind": "result", "statement": "b", "source": supplied_source(),
                       "uses": ["clm_nowhere"]}),
        ),
        down(json!({"kind": "result", "statement": "b", "source": supplied_source()})),
        Fixture::with_ctx(
            ctx().with_budget(tight),
            json!({"kind": "result", "statement": "a statement longer than five bytes",
                   "source": supplied_source()}),
        ),
        fixture(
            json!({"kind": "result", "statement": "c", "source": supplied_source(),
                       "status": "formalised"}),
        ),
        fixture(json!({"kind": "result", "statement": "d", "source": lean_source("aa")})),
    ];
    contract::check(&tool, "ledger", &fixtures).await.unwrap();
}

#[tokio::test]
async fn ledger_record_returns_the_id_and_the_status_the_evidence_earned() {
    let w = world().await;
    teach(&w, "aa");
    let tool = LedgerRecord::new(w.ledger.clone());
    assert_eq!(tool.name(), "ledger_record");

    let supplied = call(
        &tool,
        json!({"kind": "lead", "statement": "try induction",
        "source": supplied_source(), "rationale": "the base case is easy"}),
    )
    .await;
    assert_eq!(supplied.outcome, Outcome::Ok);
    assert_eq!(supplied.data["status"], "asserted");
    assert_eq!(supplied.data["created"], true);
    assert_eq!(supplied.data["kind"], "lead");

    let lean = call(
        &tool,
        json!({"kind": "result", "statement": "main",
        "source": lean_source("aa")}),
    )
    .await;
    assert_eq!(lean.data["status"], "formalised");
    assert_eq!(lean.data["evidence_origin"], "collected");
    assert_eq!(lean.data["alignment"], "not_reviewed");
    assert_eq!(lean.data["lean"]["verdict"], "checked");

    let again = call(
        &tool,
        json!({"kind": "result", "statement": "main",
        "source": lean_source("aa")}),
    )
    .await;
    assert_eq!(again.data["created"], false);
    assert_eq!(id_of(&again), id_of(&lean));

    w.lean.set(
        "lean/Main.lean",
        "Main.thm",
        Ok(LeanVerdict {
            verdict: LeanVerdictKind::CheckedWithSorry,
            sorries: 1,
            ..verdict("cc")
        }),
    );
    let sorry = call(
        &tool,
        json!({"kind": "result", "statement": "later",
        "source": lean_source("cc")}),
    )
    .await;
    assert_eq!(sorry.data["status"], "asserted");
    assert_eq!(
        sorry.unknowns.len(),
        1,
        "the reason the status is low is reported"
    );
}

#[tokio::test]
async fn a_caller_can_never_send_a_status_above_asserted() {
    let w = world().await;
    teach(&w, "aa");
    let tool = LedgerRecord::new(w.ledger.clone());
    for status in ["cited", "checked", "formalised"] {
        let sent = call(
            &tool,
            json!({"kind": "result", "statement": status,
            "source": lean_source("aa"), "status": status}),
        )
        .await;
        assert_eq!(sent.code(), Some(ErrorCode::Forbidden), "{status}");
    }
    let collected = call(
        &tool,
        json!({"kind": "result", "statement": "x",
        "source": supplied_source(), "evidence_origin": "collected"}),
    )
    .await;
    assert_eq!(collected.code(), Some(ErrorCode::Forbidden));
    assert_eq!(
        w.cortex.events().len(),
        0,
        "a refused record writes nothing"
    );
    assert_eq!(w.lean.calls().len(), 0, "and runs no check");

    let bogus = call(
        &tool,
        json!({"kind": "result", "statement": "x",
        "source": supplied_source(), "status": "proved"}),
    )
    .await;
    assert_eq!(bogus.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn asserted_and_supplied_only_pin_a_claim_down() {
    let w = world().await;
    teach(&w, "aa");
    let tool = LedgerRecord::new(w.ledger.clone());
    let pinned = call(
        &tool,
        json!({"kind": "result", "statement": "pinned",
        "source": lean_source("aa"), "status": "asserted"}),
    )
    .await;
    assert_eq!(pinned.data["status"], "asserted");
    let relabelled = call(
        &tool,
        json!({"kind": "result", "statement": "relabelled",
        "source": lean_source("aa"), "evidence_origin": "supplied"}),
    )
    .await;
    assert_eq!(relabelled.data["status"], "asserted");
    assert_eq!(relabelled.data["evidence_origin"], "supplied");
    assert_eq!(w.lean.calls().len(), 0, "no check ran for either");
}

fn arxiv_source() -> Value {
    json!({"arxiv": {"id": "2012.04125", "version": "v3", "label": "thm:main"}})
}

#[tokio::test]
async fn asserted_and_supplied_pin_an_arxiv_claim_below_cited() {
    let w = world().await;
    let tool = LedgerRecord::new(w.ledger.clone());
    let cited = call(
        &tool,
        json!({"kind": "fact", "statement": "cited", "source": arxiv_source()}),
    )
    .await;
    assert_eq!(cited.data["status"], "cited");
    let pinned = call(
        &tool,
        json!({"kind": "fact", "statement": "pinned",
        "source": arxiv_source(), "status": "asserted"}),
    )
    .await;
    assert_eq!(pinned.data["status"], "asserted");
    let relabelled = call(
        &tool,
        json!({"kind": "fact", "statement": "relabelled",
        "source": arxiv_source(), "evidence_origin": "supplied"}),
    )
    .await;
    assert_eq!(relabelled.data["status"], "asserted");
    assert_eq!(relabelled.data["evidence_origin"], "supplied");
}

#[tokio::test]
async fn an_arxiv_source_without_one_locator_is_invalid_input() {
    let w = world().await;
    let tool = LedgerRecord::new(w.ledger.clone());
    for arxiv in [
        json!({"id": "2012.04125", "version": "v3"}),
        json!({"id": "2012.04125", "version": "v3", "label": "thm:main", "section": "2"}),
    ] {
        let sent = call(
            &tool,
            json!({"kind": "fact", "statement": "x", "source": {"arxiv": arxiv}}),
        )
        .await;
        assert_eq!(sent.code(), Some(ErrorCode::InvalidInput), "{arxiv}");
    }
    let schema = tool.schema();
    let arxiv = &schema["properties"]["source"]["oneOf"][0]["properties"]["arxiv"];
    assert_eq!(
        arxiv["oneOf"],
        json!([{"required": ["label"]}, {"required": ["section"]}]),
        "the schema asks for exactly one locator"
    );
}

#[tokio::test]
async fn ledger_record_stores_reason_and_refutes_and_ledger_claims_returns_them() {
    let w = world().await;
    let target = seed(&w, "every n works").await;
    let record = LedgerRecord::new(w.ledger.clone());
    let claims = LedgerClaims::new(w.ledger.clone());

    let refutation = call(
        &record,
        json!({"kind": "refutation", "statement": "n = 7 fails",
        "source": supplied_source(), "refutes": target}),
    )
    .await;
    assert_eq!(refutation.data["kind"], "refutation");
    let read = call(&claims, json!({"id": id_of(&refutation)})).await;
    assert_eq!(read.data["claim"]["refutes"], json!(target));

    let dead_end = call(
        &record,
        json!({"kind": "dead_end", "statement": "induction on n",
        "source": supplied_source(), "reason": "the step needs n + 2"}),
    )
    .await;
    assert_eq!(dead_end.data["kind"], "dead_end");
    let read = call(&claims, json!({"id": id_of(&dead_end)})).await;
    assert_eq!(read.data["claim"]["reason"], "the step needs n + 2");
}

#[tokio::test]
async fn ledger_record_refuses_a_uses_that_repeats_an_id() {
    let w = world().await;
    let a = seed(&w, "a").await;
    let record = LedgerRecord::new(w.ledger.clone());
    let written = w.cortex.events().len();
    let twice = call(
        &record,
        json!({"kind": "result", "statement": "b", "source": supplied_source(),
        "uses": [a, a]}),
    )
    .await;
    assert_eq!(twice.code(), Some(ErrorCode::InvalidInput));
    assert_eq!(w.cortex.events().len(), written);
}

#[tokio::test]
async fn a_lean_timeout_is_unknown_not_an_error() {
    let w = world().await;
    w.lean.set(
        "lean/Main.lean",
        "Main.thm",
        Err(crate::Error::EvidenceTimeout),
    );
    let tool = LedgerRecord::new(w.ledger.clone());
    let timed_out = call(
        &tool,
        json!({"kind": "result", "statement": "slow",
        "source": lean_source("aa")}),
    )
    .await;
    assert_eq!(timed_out.outcome, Outcome::Unknown);
    assert_eq!(w.cortex.events().len(), 0);
}

#[tokio::test]
async fn a_tool_source_is_checked_against_the_call_log_through_the_tool() {
    let w = world().await;
    let tool = LedgerRecord::new(w.ledger.clone());
    let source = json!({"tool": {"tool": "py_run", "call_id": "c-1"}});
    let forged = call(
        &tool,
        json!({"kind": "result", "statement": "p", "source": source,
        "frame": "n < 100"}),
    )
    .await;
    assert_eq!(forged.code(), Some(ErrorCode::Forbidden));

    w.calls.record(CallRecord::new(
        "c-1",
        "py_run",
        "sendov",
        &Envelope::ok(json!({})),
        1,
    ));
    let real = call(
        &tool,
        json!({"kind": "result", "statement": "p", "source": source,
        "frame": "n < 100"}),
    )
    .await;
    assert_eq!(real.data["status"], "checked");
    let frameless = call(
        &tool,
        json!({"kind": "result", "statement": "q", "source": source}),
    )
    .await;
    assert_eq!(frameless.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn ledger_claims_keeps_the_tool_contract() {
    let w = world().await;
    let id = seed(&w, "n^2 is even").await;
    let tool = LedgerClaims::new(w.ledger.clone());
    let one_row = Budget {
        rows: 1,
        ..Budget::default()
    };
    let fixtures = [
        fixture(json!({})),
        fixture(json!({"id": id})),
        fixture(json!({"id": id, "kind": "result"})),
        fixture(json!({"id": "clm_nowhere"})),
        down(json!({})),
        Fixture::with_ctx(ctx().with_budget(one_row), json!({"limit": 5})),
    ];
    contract::check(&tool, "ledger", &fixtures).await.unwrap();
}

#[tokio::test]
async fn ledger_claims_filters_pages_and_trims() {
    let w = world().await;
    teach(&w, "aa");
    let record = LedgerRecord::new(w.ledger.clone());
    for n in 0..4 {
        call(
            &record,
            json!({"kind": "lead", "statement": format!("lead {n}"),
            "source": supplied_source()}),
        )
        .await;
    }
    let long = "w".repeat(500);
    call(
        &record,
        json!({"kind": "result", "statement": long, "source": lean_source("aa")}),
    )
    .await;
    let claims = LedgerClaims::new(w.ledger.clone());

    let all = call(&claims, json!({"limit": 2})).await;
    let rows = all.data["claims"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    // Rows keep recall order; the stub recalls its newest notes first.
    assert_eq!(rows[0]["kind"], "result", "recall order");
    assert_eq!(rows[0]["statement"].as_str().unwrap().chars().count(), 301);
    assert_eq!(rows[0]["status"], "formalised");
    assert_eq!(rows[0]["alignment"], "not_reviewed");
    assert_eq!(rows[1]["statement"], "lead 3");
    let cursor = all.cursor.clone().unwrap();

    let next = call(&claims, json!({"limit": 2, "cursor": cursor})).await;
    assert_eq!(next.data["claims"][0]["statement"], "lead 2");
    let last = call(
        &claims,
        json!({"limit": 2, "cursor": next.cursor.clone().unwrap()}),
    )
    .await;
    assert_eq!(last.data["claims"].as_array().unwrap().len(), 1);
    assert!(last.cursor.is_none());

    let leads = call(&claims, json!({"kind": "lead", "limit": 50})).await;
    assert_eq!(leads.data["claims"].as_array().unwrap().len(), 4);
    let formal = call(&claims, json!({"min_status": "formalised"})).await;
    assert_eq!(formal.data["claims"].as_array().unwrap().len(), 1);
    let none = call(&claims, json!({"alignment": "disputed"})).await;
    assert_eq!(none.data["claims"].as_array().unwrap().len(), 0);
    let queried = call(&claims, json!({"query": "  lead  "})).await;
    assert_eq!(queried.outcome, Outcome::Ok);
    let blank = call(&claims, json!({"query": "  "})).await;
    assert_eq!(blank.outcome, Outcome::Ok);

    let bad = call(&claims, json!({"cursor": "x"})).await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn ledger_claims_refuses_id_with_any_filter_before_the_budget() {
    let w = world().await;
    let id = seed(&w, "n^2 is even").await;
    let claims = LedgerClaims::new(w.ledger.clone());
    let no_rows = ctx().with_budget(Budget {
        rows: 0,
        ..Budget::default()
    });
    for (field, value) in [
        ("kind", json!("result")),
        ("cursor", json!("0")),
        ("min_status", json!("asserted")),
        ("alignment", json!("not_reviewed")),
        ("query", json!("even")),
    ] {
        let mut args = json!({"id": id});
        args[field] = value;
        let answer = claims.call(&no_rows, args).await;
        assert_eq!(answer.code(), Some(ErrorCode::InvalidInput), "{field}");
    }
    let bad_cursor = claims.call(&no_rows, json!({"cursor": "x"})).await;
    assert_eq!(bad_cursor.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn ledger_claims_caps_the_limit_at_fifty_and_pages_past_the_end_to_nothing() {
    let w = world().await;
    for n in 0..51 {
        seed(&w, &format!("claim {n}")).await;
    }
    let claims = LedgerClaims::new(w.ledger.clone());
    let capped = call(&claims, json!({"limit": 500})).await;
    assert_eq!(capped.data["claims"].as_array().unwrap().len(), 50);
    assert_eq!(capped.cursor.as_deref(), Some("50"));

    let past = call(&claims, json!({"cursor": "100"})).await;
    assert_eq!(past.outcome, Outcome::Ok);
    assert_eq!(past.data["claims"], json!([]));
    assert!(past.cursor.is_none());
}

#[tokio::test]
async fn ledger_claims_reports_effective_status_and_what_limits_it() {
    let w = world().await;
    teach(&w, "aa");
    let weak = seed(&w, "lemma").await;
    let record = LedgerRecord::new(w.ledger.clone());
    let main = call(
        &record,
        json!({"kind": "result", "statement": "main",
        "source": lean_source("aa"), "uses": [weak]}),
    )
    .await;
    let claims = LedgerClaims::new(w.ledger.clone());

    let one = call(&claims, json!({"id": id_of(&main)})).await;
    assert_eq!(one.data["claim"]["status"], "formalised");
    assert_eq!(one.data["effective_status"], "asserted");
    assert_eq!(one.data["limited_by"], json!([id_of(&main), weak]));

    let listed = call(&claims, json!({"min_status": "formalised"})).await;
    assert_eq!(
        listed.data["claims"].as_array().unwrap().len(),
        0,
        "a claim resting on an asserted one is not formalised"
    );

    teach(&w, "bb");
    let stale = call(&claims, json!({"id": id_of(&main)})).await;
    assert!(
        stale.unknowns.iter().any(|u| u.contains("changed")),
        "{:?}",
        stale.unknowns
    );
}

#[tokio::test]
async fn ledger_claims_says_when_the_window_may_hide_entries() {
    let w = world().await;
    let narrow = Arc::new(Ledger::clone(&w.ledger).with_window(1));
    seed(&w, "one").await;
    seed(&w, "two").await;
    let listed = call(&LedgerClaims::new(narrow), json!({})).await;
    assert!(listed.unknowns[0].contains("window"));
}

#[tokio::test]
async fn a_full_window_warns_even_when_revisions_collapse_to_fewer_claims() {
    let w = world().await;
    let narrow = Arc::new(Ledger::clone(&w.ledger).with_window(3));
    let one = seed(&w, "one").await;
    let two = seed(&w, "two").await;
    w.ledger.link(&ctx(), &two, &one).await.unwrap();
    // Three notes (two claims and a revision of one) fit a window of three
    // exactly: a listing knows nothing was left out.
    let listed = call(&LedgerClaims::new(narrow.clone()), json!({})).await;
    assert_eq!(listed.data["claims"].as_array().unwrap().len(), 2);
    assert!(listed.unknowns.is_empty(), "{:?}", listed.unknowns);

    // A fourth note does not fit, and the read says so even though the three
    // it did read collapse to fewer claims.
    seed(&w, "three").await;
    let listed = call(&LedgerClaims::new(narrow), json!({})).await;
    assert!(
        listed.unknowns.iter().any(|u| u.contains("window")),
        "four notes overflow a window of three: {:?}",
        listed.unknowns
    );

    let roomy = Arc::new(Ledger::clone(&w.ledger).with_window(4));
    let listed = call(&LedgerClaims::new(roomy), json!({})).await;
    assert!(listed.unknowns.is_empty(), "{:?}", listed.unknowns);
}

#[tokio::test]
async fn ledger_link_keeps_the_tool_contract() {
    let w = world().await;
    let a = seed(&w, "a").await;
    let b = seed(&w, "b").await;
    let tool = LedgerLink::new(w.ledger.clone());
    let fixtures = [
        fixture(json!({"from": a, "to": b})),
        fixture(json!({"from": a, "to": a})),
        fixture(json!({"from": a, "to": "clm_nowhere"})),
        down(json!({"from": a, "to": b})),
    ];
    contract::check(&tool, "ledger", &fixtures).await.unwrap();
}

#[tokio::test]
async fn ledger_link_records_the_edge_and_refuses_a_cycle() {
    let w = world().await;
    let a = seed(&w, "a").await;
    let b = seed(&w, "b").await;
    let tool = LedgerLink::new(w.ledger.clone());
    let linked = call(&tool, json!({"from": a, "to": b})).await;
    assert_eq!(linked.data["uses"], json!([b]));
    assert_eq!(linked.data["created"], true);
    let again = call(&tool, json!({"from": a, "to": b})).await;
    assert_eq!(again.data["created"], false);
    let cycle = call(&tool, json!({"from": b, "to": a})).await;
    assert_eq!(cycle.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn ledger_record_refuses_a_cycle_with_the_same_code_as_ledger_link() {
    let w = world().await;
    let a = seed(&w, "a").await;
    let record = LedgerRecord::new(w.ledger.clone());
    let b = call(
        &record,
        json!({"kind": "result", "statement": "b", "source": supplied_source(), "uses": [a]}),
    )
    .await;
    let cycle = call(
        &record,
        json!({"kind": "result", "statement": "a", "source": {"supplied": "s"},
        "uses": [id_of(&b)]}),
    )
    .await;
    assert_eq!(cycle.code(), Some(ErrorCode::InvalidInput));
    let linked = call(
        &LedgerLink::new(w.ledger.clone()),
        json!({"from": a, "to": id_of(&b)}),
    )
    .await;
    assert_eq!(linked.code(), cycle.code());
}

#[tokio::test]
async fn ledger_bound_keeps_the_tool_contract() {
    let w = world().await;
    teach(&w, "bb");
    let tool = LedgerBound::new(w.ledger.clone());
    let base = |source: Value| json!({"quantity": "radius", "direction": "upper", "value": 4, "source": source});
    let fixtures = [
        fixture(base(supplied_source())),
        fixture(
            json!({"quantity": " ", "direction": "upper", "value": 4, "source": supplied_source()}),
        ),
        fixture(base(
            json!({"lean": {"path": "lean/Gone.lean", "declaration": "G", "file_sha256": "x"}}),
        )),
        down(base(supplied_source())),
        fixture(base(
            json!({"tool": {"tool": "py_run", "call_id": "never-ran"}}),
        )),
        fixture(base(lean_source("stale"))),
    ];
    contract::check(&tool, "ledger", &fixtures).await.unwrap();
}

#[tokio::test]
async fn ledger_bound_returns_the_best_bound_either_way() {
    let w = world().await;
    let tool = LedgerBound::new(w.ledger.clone());
    let offer = |value: Value, text: &str| {
        json!({"quantity": "Sendov radius",
        "direction": "upper", "value": value, "source": {"supplied": text}})
    };

    let first = call(&tool, offer(json!(2.5), "a")).await;
    assert_eq!(first.data["improved"], true);
    assert_eq!(first.data["recorded"]["kind"], "bound");
    assert_eq!(first.data["best"]["value"], 2.5);

    let worse = call(&tool, offer(json!(3), "b")).await;
    assert_eq!(worse.data["improved"], false);
    assert_eq!(worse.data["recorded"]["kind"], "fact");
    assert_eq!(worse.data["best"]["value"], 2.5);
    assert_eq!(worse.data["best"]["status"], "asserted");

    let log = Arc::new(InMemoryCallLog::new());
    log.record(CallRecord::new(
        "c",
        "py_run",
        "sendov",
        &Envelope::ok(json!({})),
        1,
    ));
    let w2 = World {
        ledger: Arc::new(Ledger::new(w.memory.clone(), w.lean.clone(), log)),
        ..world().await
    };
    let tool2 = LedgerBound::new(w2.ledger.clone());
    let tooled = call(
        &tool2,
        json!({"quantity": "Sendov radius", "direction": "upper",
        "value": 1, "frame": "degree <= 9",
        "source": {"tool": {"tool": "py_run", "call_id": "c"}}}),
    )
    .await;
    assert_eq!(tooled.data["recorded"]["status"], "checked");
}

fn radius(value: &Value, source: &Value) -> Value {
    json!({"quantity": "Sendov radius", "direction": "upper", "value": value, "source": source})
}

#[tokio::test]
async fn ledger_bound_warns_when_a_full_window_may_hide_an_older_best() {
    let w = world().await;
    let narrow = LedgerBound::new(Arc::new(Ledger::clone(&w.ledger).with_window(2)));
    let first = call(&narrow, radius(&json!(3), &json!({"supplied": "a"}))).await;
    assert!(first.unknowns.is_empty(), "{:?}", first.unknowns);
    call(&narrow, radius(&json!(2), &json!({"supplied": "b"}))).await;
    // Two bounds fit the window of two; the third offer reads both.
    let third = call(&narrow, radius(&json!(4), &json!({"supplied": "c"}))).await;
    assert!(third.unknowns.is_empty(), "{:?}", third.unknowns);
    // With three recorded, a read of two leaves one out and says so.
    let fourth = call(&narrow, radius(&json!(5), &json!({"supplied": "d"}))).await;
    assert!(
        fourth.unknowns.iter().any(|u| u.contains("window")),
        "{:?}",
        fourth.unknowns
    );
}

#[tokio::test]
async fn ledger_bound_reports_the_best_bounds_effective_status() {
    let w = world().await;
    teach(&w, "aa");
    let tool = LedgerBound::new(w.ledger.clone());
    let formal = call(&tool, radius(&json!(2), &lean_source("aa"))).await;
    assert_eq!(formal.data["best"]["status"], "formalised");

    teach(&w, "bb");
    let worse = call(&tool, radius(&json!(3), &supplied_source())).await;
    assert_eq!(worse.data["improved"], false);
    assert_eq!(worse.data["best"]["id"], formal.data["best"]["id"]);
    assert_eq!(
        worse.data["best"]["status"], "asserted",
        "the best bound's file changed, so it stands as asserted"
    );
    assert!(
        worse.unknowns.iter().any(|u| u.contains("changed")),
        "{:?}",
        worse.unknowns
    );
}

#[tokio::test]
async fn a_lean_timeout_through_ledger_bound_is_unknown() {
    let w = world().await;
    w.lean.set(
        "lean/Main.lean",
        "Main.thm",
        Err(crate::Error::EvidenceTimeout),
    );
    let tool = LedgerBound::new(w.ledger.clone());
    let timed_out = call(&tool, radius(&json!(2), &lean_source("aa"))).await;
    assert_eq!(timed_out.outcome, Outcome::Unknown);
    assert_eq!(w.cortex.events().len(), 0, "nothing was recorded");
}

#[tokio::test]
async fn no_tool_can_raise_a_claim_through_its_arguments() {
    let w = world().await;
    teach(&w, "aa");
    let a = seed(&w, "a").await;
    let b = seed(&w, "b").await;
    let stored = w.cortex.events().len();
    let registry = {
        let mut registry = ToolRegistry::new();
        for tool in tools(&w.memory, &w.ledger) {
            registry.register(tool).unwrap();
        }
        registry
    };
    let smuggled = [
        ("recall_memory", json!({"query": "q"})),
        ("ledger_claims", json!({})),
        ("ledger_link", json!({"from": a, "to": b})),
        (
            "ledger_bound",
            json!({"quantity": "q", "direction": "upper", "value": 1,
            "source": lean_source("aa")}),
        ),
    ];
    for (name, mut args) in smuggled {
        args["status"] = json!("formalised");
        args["evidence_origin"] = json!("collected");
        let tool = registry.get(name).unwrap();
        let answer = tool.call(&ctx(), args).await;
        assert_eq!(answer.code(), Some(ErrorCode::InvalidInput), "{name}");
    }
    assert_eq!(w.cortex.events().len(), stored, "nothing was written");
    assert_eq!(registry.names().count(), 5);
}

#[tokio::test]
async fn the_toolset_declares_the_catalogue_exposure_and_permissions() {
    let w = world().await;
    let all = tools(&w.memory, &w.ledger);
    let summary: Vec<(&str, bool, bool)> = all
        .iter()
        .map(|t| {
            (
                t.name(),
                t.exposure() == supermath_tool::Exposure::Direct,
                t.permission() == supermath_tool::Permission::Write,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("recall_memory", true, false),
            ("ledger_record", true, true),
            ("ledger_claims", false, false),
            ("ledger_link", false, true),
            ("ledger_bound", false, true),
        ]
    );
    for tool in &all {
        assert!(!tool.kind().is_agentic());
        assert_ne!(tool.description().len(), 0);
    }
}

#[test]
fn trim_cuts_on_characters_and_marks_the_cut() {
    assert_eq!(trim("short", 10), "short");
    assert_eq!(trim("αβγδ", 2), "αβ…");
}
