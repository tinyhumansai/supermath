//! The ports, wired to the right domains, through the public `Toolkit`
//! against a loopback Lean service and a fake `CortexDB`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use serde_json::{Value, json};
use supermath_files::ProjectPath;
use supermath_tool::{Envelope, ErrorCode, Outcome};
use support::{CLEAN_LEAN, Env, LEAN_PATH, LeanService, cortex};

/// A toolkit whose Lean service and `CortexDB` are stubs, with the sample
/// proof already written to the project.
async fn rig() -> (supermath_mcp::Toolkit, LeanService, Env, String) {
    let lean = LeanService::start().await;
    let cortex = cortex().await;
    let env = Env::new()
        .set("LEAN_URL", &lean.stub.base)
        .set("CORTEX_API_URL", cortex.url())
        .set("CORTEX_API_KEY", "key");
    env.put(LEAN_PATH, CLEAN_LEAN);
    let toolkit = env.toolkit();
    let sha = toolkit
        .dispatch("file_read", json!({"path": LEAN_PATH}))
        .await
        .envelope
        .data["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    (toolkit, lean, env, sha)
}

fn record_args(sha: &str) -> Value {
    json!({"kind": "result", "statement": "two plus two is four",
        "source": {"lean": {"path": LEAN_PATH, "declaration": "two_add_two", "file_sha256": sha}}})
}

async fn record(toolkit: &supermath_mcp::Toolkit, args: Value) -> Envelope {
    toolkit.dispatch("ledger_record", args).await.envelope
}

#[tokio::test]
async fn a_clean_lean_check_is_what_promotes_a_lean_claim() {
    let (toolkit, lean, _env, sha) = rig().await;
    let envelope = record(&toolkit, record_args(&sha)).await;
    assert_eq!(envelope.outcome, Outcome::Ok, "{envelope:?}");
    assert_eq!(envelope.data["status"], "formalised");
    assert_eq!(envelope.data["evidence_origin"], "collected");
    assert_eq!(
        lean.stub.count("/check"),
        1,
        "the ledger ran the Lean check"
    );
}

#[tokio::test]
async fn a_sorry_in_the_check_leaves_the_claim_asserted() {
    let (toolkit, lean, _env, sha) = rig().await;
    lean.answer(json!({"status": "ok", "toolchain": "t", "mathlib": "m", "elapsed_ms": 1,
        "messages": [{"severity": "warning", "line": 3, "column": 8, "text": "declaration uses 'sorry'"}],
        "sorries": [{"line": 4, "column": 2, "goal": "⊢ 2 + 2 = 4"}],
        "axioms": {"two_add_two": ["propext", "sorryAx"]}}));
    let envelope = record(&toolkit, record_args(&sha)).await;
    assert_eq!(envelope.data["status"], "asserted", "{envelope:?}");
}

#[tokio::test]
async fn arguments_cannot_promote_a_claim_the_check_would_not() {
    let (toolkit, lean, _env, sha) = rig().await;
    let mut forged = record_args(&sha);
    forged["status"] = json!("formalised");
    let envelope = record(&toolkit, forged).await;
    assert_eq!(envelope.code(), Some(ErrorCode::Forbidden));
    let mut claimed = record_args(&sha);
    claimed["evidence_origin"] = json!("collected");
    assert_eq!(
        record(&toolkit, claimed).await.code(),
        Some(ErrorCode::Forbidden)
    );
    assert_eq!(
        lean.stub.count("/check"),
        0,
        "a forged call never reaches Lean"
    );
}

#[tokio::test]
async fn a_changed_file_is_refused_as_stale() {
    let (toolkit, _lean, env, sha) = rig().await;
    env.put(LEAN_PATH, &format!("{CLEAN_LEAN}\n-- edited\n"));
    let envelope = record(&toolkit, record_args(&sha)).await;
    assert_eq!(envelope.code(), Some(ErrorCode::Conflict), "{envelope:?}");
}

#[tokio::test]
async fn a_missing_file_or_declaration_is_not_found() {
    let (toolkit, _lean, _env, sha) = rig().await;
    let mut missing_decl = record_args(&sha);
    missing_decl["source"]["lean"]["declaration"] = json!("no_such_theorem");
    assert_eq!(
        record(&toolkit, missing_decl).await.code(),
        Some(ErrorCode::NotFound)
    );
    let mut missing_file = record_args(&sha);
    missing_file["source"]["lean"]["path"] = json!("lean/Supermath/Gone.lean");
    assert_eq!(
        record(&toolkit, missing_file).await.code(),
        Some(ErrorCode::NotFound)
    );
    let mut bad_path = record_args(&sha);
    bad_path["source"]["lean"]["path"] = json!("../etc/passwd");
    assert_eq!(
        record(&toolkit, bad_path).await.code(),
        Some(ErrorCode::NotFound)
    );
}

#[tokio::test]
async fn a_lean_timeout_is_unknown_not_a_verdict() {
    let (toolkit, lean, _env, sha) = rig().await;
    lean.answer(
        json!({"status": "timeout", "toolchain": "t", "mathlib": "m", "elapsed_ms": 60000}),
    );
    let envelope = record(&toolkit, record_args(&sha)).await;
    assert_eq!(envelope.outcome, Outcome::Unknown, "{envelope:?}");
}

#[tokio::test]
async fn a_lean_service_that_is_down_is_backend_unavailable() {
    let down = Env::new()
        .set("LEAN_URL", "http://127.0.0.1:1")
        .set("CORTEX_API_URL", cortex().await.url())
        .set("CORTEX_API_KEY", "key");
    down.put(LEAN_PATH, CLEAN_LEAN);
    let toolkit = down.toolkit();
    let envelope = record(&toolkit, record_args("any")).await;
    assert_eq!(
        envelope.code(),
        Some(ErrorCode::BackendUnavailable),
        "{envelope:?}"
    );
}

#[tokio::test]
async fn no_lean_url_leaves_lean_check_unavailable_and_the_catalogue_unchanged() {
    let (configured, _lean, _env, _sha) = rig().await;
    let bare = Env::new();
    bare.put(LEAN_PATH, CLEAN_LEAN);
    let unconfigured = bare.toolkit();
    let names = |t: &supermath_mcp::Toolkit| t.registry().names().collect::<Vec<_>>();
    assert_eq!(names(&configured), names(&unconfigured));
    let envelope = unconfigured
        .dispatch("lean_check", json!({"path": LEAN_PATH}))
        .await
        .envelope;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn the_lean_verdict_shows_in_file_list_through_the_lean_observer() {
    let (toolkit, _lean, _env, sha) = rig().await;
    toolkit
        .dispatch("lean_check", json!({"path": LEAN_PATH}))
        .await;
    let listing = toolkit.dispatch("file_list", json!({})).await.envelope;
    let row = listing.data["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == LEAN_PATH)
        .unwrap()
        .clone();
    assert_eq!(row["last_result"], "checked", "{row}");
    assert_eq!(row["result_sha256"], json!(sha));
    assert_eq!(row["stale"], false);
}

#[tokio::test]
async fn the_python_run_log_shows_in_file_list_through_the_python_observer() {
    let env = Env::new();
    env.put("python/work/s.py", "import math\n");
    let toolkit = env.toolkit();
    let path = ProjectPath::parse("python/work/s.py").unwrap();
    let sha = toolkit
        .dispatch("file_read", json!({"path": "python/work/s.py"}))
        .await
        .envelope
        .data["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    toolkit
        .python()
        .observer()
        .record("demo", &path, "ok", &sha);
    let listing = toolkit.dispatch("file_list", json!({})).await.envelope;
    let row = &listing.data["files"][0];
    assert_eq!(row["last_result"], "ok", "{row}");
    assert_eq!(row["stale"], false);
}

#[tokio::test]
async fn a_tool_source_is_checked_against_the_shared_call_log() {
    let env = Env::new()
        .set("CORTEX_API_URL", cortex().await.url())
        .set("CORTEX_API_KEY", "key");
    let toolkit = env.toolkit();
    let ran = toolkit.dispatch("calc_factor", json!({"n": 360})).await;
    let args = |call_id: &str, tool: &str| {
        json!({"kind": "result", "statement": "360 = 2^3 * 3^2 * 5", "frame": "n = 360",
            "source": {"tool": {"tool": tool, "call_id": call_id}}})
    };
    let earned = record(&toolkit, args(&ran.call_id, "calc_factor")).await;
    assert_eq!(earned.data["status"], "checked", "{earned:?}");
    let invented = record(&toolkit, args("call-999", "calc_factor")).await;
    assert!(
        !invented.is_ok(),
        "an id the log never recorded earns nothing: {invented:?}"
    );
    let wrong_tool = record(&toolkit, args(&ran.call_id, "calc_gcd_lcm")).await;
    assert!(!wrong_tool.is_ok(), "{wrong_tool:?}");
}

#[tokio::test]
async fn a_tool_source_from_another_process_is_found_in_the_call_log_on_disk() {
    let env = Env::new()
        .set("CORTEX_API_URL", cortex().await.url())
        .set("CORTEX_API_KEY", "key");
    // Two persistent toolkits stand in for two `supermath-mcp call` runs.
    let first = supermath_mcp::Toolkit::compose_persistent(&env.config(), "demo").unwrap();
    let ran = first.dispatch("calc_factor", json!({"n": 360})).await;
    drop(first);
    let second = supermath_mcp::Toolkit::compose_persistent(&env.config(), "demo").unwrap();
    let earned = record(
        &second,
        json!({"kind": "result", "statement": "360 = 2^3 * 3^2 * 5", "frame": "n = 360",
            "source": {"tool": {"tool": "calc_factor", "call_id": ran.call_id}}}),
    )
    .await;
    assert_eq!(earned.data["status"], "checked", "{earned:?}");
    // The in-memory log of a fresh process knows nothing of that call.
    let forgetful = env.toolkit();
    let refused = record(
        &forgetful,
        json!({"kind": "result", "statement": "360 = 2^3 * 3^2 * 5", "frame": "n = 360",
            "source": {"tool": {"tool": "calc_factor", "call_id": ran.call_id}}}),
    )
    .await;
    assert!(!refused.is_ok(), "{refused:?}");
}

#[tokio::test]
async fn lean_blueprint_names_the_claim_recorded_for_a_declaration() {
    let (toolkit, _lean, _env, sha) = rig().await;
    let before = toolkit.dispatch("lean_blueprint", json!({})).await.envelope;
    assert_eq!(
        before.data["nodes"][0]["claim_id"],
        Value::Null,
        "{before:?}"
    );
    let recorded = record(&toolkit, record_args(&sha)).await;
    let id = recorded.data["id"].as_str().unwrap().to_owned();
    let after = toolkit.dispatch("lean_blueprint", json!({})).await.envelope;
    assert_eq!(after.data["nodes"][0]["claim_id"], json!(id), "{after:?}");
}

#[tokio::test]
async fn every_dispatch_lands_in_one_call_log() {
    let (toolkit, _lean, _env, _sha) = rig().await;
    toolkit
        .dispatch("lean_check", json!({"path": LEAN_PATH}))
        .await;
    toolkit
        .dispatch("calc_gcd_lcm", json!({"a": 4, "b": 6}))
        .await;
    let tools: Vec<String> = toolkit
        .call_log()
        .all()
        .into_iter()
        .map(|r| r.tool)
        .collect();
    assert!(tools.contains(&"lean_check".to_owned()) && tools.contains(&"calc_gcd_lcm".to_owned()));
}
