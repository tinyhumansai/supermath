//! `lean_build` and `lean_blueprint` over a three-file project:
//! `Base` (clean) <- `Mid` (a sorry) <- `Top` (rests on it).

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use std::sync::Arc;

use serde_json::{Value, json};
use supermath_lean::{ClaimIndex, Lean};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{ErrorCode, Outcome, Tool};
use support::{Env, Stub, config, ctx, dead_url, down_ctx, service};

const BASE: &str = "import Mathlib\n\ntheorem base_fact : True := trivial\n";
const MID: &str = "import Supermath.Base\n\ntheorem mid_fact : True := by\n  sorry\n";
const TOP: &str = "import Supermath.Mid\n\ntheorem top_claim : True := mid_fact\n";

async fn project() -> (Stub, Env) {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Base.lean", BASE);
    env.put("lean/Supermath/Mid.lean", MID);
    env.put("lean/Supermath/Top.lean", TOP);
    (stub, env)
}

fn files(envelope: &supermath_tool::Envelope) -> Vec<Value> {
    envelope.data["files"].as_array().unwrap().clone()
}

// --- lean_build ------------------------------------------------------------------

#[tokio::test]
async fn build_checks_in_dependency_order_and_finds_cross_file_sorries() {
    let (stub, env) = project().await;
    let envelope = env.call("lean_build", json!({})).await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    let rows = files(&envelope);
    let verdicts: Vec<_> = rows
        .iter()
        .map(|r| (r["path"].as_str().unwrap(), r["verdict"].as_str().unwrap()))
        .collect();
    assert_eq!(
        verdicts,
        [
            ("lean/Supermath/Base.lean", "checked"),
            ("lean/Supermath/Mid.lean", "checked_with_sorry"),
            ("lean/Supermath/Top.lean", "checked_with_sorry"),
        ]
    );
    assert_eq!(rows[1]["sorry_count"], 1);
    assert_eq!(
        rows[1]["depends_on_sorry"],
        json!([]),
        "Mid's sorry is its own"
    );
    assert_eq!(rows[2]["depends_on_sorry"], json!(["top_claim"]));
    assert_eq!(rows[0]["cached"], false);
    assert_eq!(
        envelope.data["summary"],
        json!({"checked": 1, "checked_with_sorry": 2, "failed": 0, "unknown": 0})
    );
    assert_eq!(envelope.data["total"], 3);
    let order: Vec<_> = stub
        .requests()
        .iter()
        .map(|r| r.body["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        order,
        [
            "lean/Supermath/Base.lean",
            "lean/Supermath/Mid.lean",
            "lean/Supermath/Top.lean"
        ]
    );
}

#[tokio::test]
async fn a_second_build_reuses_every_unchanged_verdict() {
    let (stub, env) = project().await;
    env.call("lean_build", json!({})).await;
    let again = env.call("lean_build", json!({})).await;
    assert!(files(&again).iter().all(|r| r["cached"] == true));
    assert_eq!(stub.count("/check"), 3);
    env.put("lean/Supermath/Base.lean", &format!("{BASE}-- edit\n"));
    let third = env.call("lean_build", json!({})).await;
    let cached: Vec<_> = files(&third).iter().map(|r| r["cached"].clone()).collect();
    // Base changed, and Mid and Top import it (transitively), so all three rerun.
    assert_eq!(cached, [json!(false), json!(false), json!(false)]);
    assert_eq!(stub.count("/check"), 6);
}

#[tokio::test]
async fn build_pages_its_rows() {
    let (_stub, env) = project().await;
    let first = env.call("lean_build", json!({"limit": 2})).await;
    assert_eq!(files(&first).len(), 2);
    let cursor = first.cursor.clone().unwrap();
    let second = env
        .call("lean_build", json!({"limit": 2, "cursor": cursor}))
        .await;
    assert_eq!(files(&second).len(), 1);
    assert!(second.cursor.is_none());
    let bad = env.call("lean_build", json!({"cursor": "x"})).await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn build_reports_failures_and_unknowns_per_file() {
    let (_stub, env) = project().await;
    env.put(
        "lean/Supermath/Slow.lean",
        "import Mathlib\n\ntheorem slow_claim : True := trivial\n",
    );
    env.put(
        "lean/Supermath/Gone.lean",
        "import Mathlib\n\ntheorem gone_claim : True := trivial\n",
    );
    env.put(
        "lean/Supermath/Broken.lean",
        "import Mathlib\n\ntheorem broken_claim : False := by\n  rfl\n",
    );
    let envelope = env.call("lean_build", json!({})).await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    let rows = files(&envelope);
    let by = |name: &str| {
        rows.iter()
            .find(|r| r["path"] == format!("lean/Supermath/{name}.lean"))
            .unwrap()
    };
    assert_eq!(by("Broken")["verdict"], "failed");
    assert_eq!(by("Broken")["error_count"], 1);
    assert_eq!(by("Slow")["verdict"], "unknown");
    assert!(
        by("Slow")["reason"]
            .as_str()
            .unwrap()
            .contains("ran out of time")
    );
    assert_eq!(by("Gone")["verdict"], "unknown");
    assert!(by("Gone")["reason"].as_str().unwrap().contains("not found"));
    assert_eq!(envelope.data["summary"]["unknown"], 2);
}

#[tokio::test]
async fn an_exhausted_build_budget_leaves_the_rest_unknown() {
    let stub = service().await;
    let mut cfg = config(Some(&stub.base));
    cfg.build_timeout_ms = 0;
    let env = Env::new(cfg);
    env.put("lean/Supermath/Base.lean", BASE);
    let envelope = env.call("lean_build", json!({})).await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(files(&envelope)[0]["verdict"], "unknown");
    assert!(envelope.unknowns.iter().any(|u| u.contains("budget")));
    assert_eq!(stub.count("/check"), 0);
}

#[tokio::test]
async fn build_with_the_service_down_is_backend_unavailable_and_an_empty_project_is_fine() {
    let down = Env::new(config(Some(&dead_url().await)));
    down.put("lean/Supermath/Base.lean", BASE);
    assert_eq!(
        down.call("lean_build", json!({})).await.code(),
        Some(ErrorCode::BackendUnavailable)
    );
    let empty = Env::new(config(None));
    let envelope = empty.call("lean_build", json!({})).await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(envelope.data["total"], 0);
}

#[tokio::test]
async fn lean_build_passes_the_contract_checker() {
    let (_stub, env) = project().await;
    env.put_in("downproj", "lean/Supermath/Base.lean", BASE);
    let fixtures = [
        Fixture::with_ctx(ctx(), json!({})),
        Fixture::with_ctx(ctx(), json!({"cursor": "nope"})),
        Fixture::with_ctx(down_ctx(), json!({})),
    ];
    let tool = env.tool("lean_build");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
}

// --- lean_blueprint ------------------------------------------------------------------

struct Claims;

impl ClaimIndex for Claims {
    fn claim_id(&self, project: &str, _path: &str, declaration: &str) -> Option<String> {
        (project == "demo" && declaration == "top_claim").then(|| "claim-7".to_owned())
    }
}

fn node<'a>(envelope: &'a supermath_tool::Envelope, name: &str) -> &'a Value {
    envelope.data["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == name)
        .unwrap_or_else(|| panic!("no node {name}"))
}

#[tokio::test]
async fn blueprint_graphs_declarations_with_status_edges_and_what_they_rest_on() {
    let (stub, env) = project().await;
    let envelope = env.call("lean_blueprint", json!({})).await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(envelope.data["total"], 3);
    let base = node(&envelope, "base_fact");
    assert_eq!(base["status"], "checked");
    assert_eq!(base["uses"], json!([]));
    assert_eq!(base["rests_on_sorry"], json!([]));
    let mid = node(&envelope, "mid_fact");
    assert_eq!(mid["status"], "checked_with_sorry");
    assert_eq!(mid["uses"], json!(["base_fact"]));
    let top = node(&envelope, "top_claim");
    assert_eq!(top["status"], "checked_with_sorry");
    assert_eq!(
        top["uses"],
        json!(["base_fact", "mid_fact"]),
        "Mathlib and self edges are dropped"
    );
    assert_eq!(top["rests_on_sorry"], json!(["mid_fact"]));
    assert_eq!(top["path"], "lean/Supermath/Top.lean");
    assert_eq!(top["line"], 3);
    assert_eq!(top["claim_id"], Value::Null);
    assert_eq!(stub.last_body("/check")["uses"], true);
}

#[tokio::test]
async fn blueprint_with_a_root_shows_only_what_it_rests_on() {
    let (_stub, env) = project().await;
    let envelope = env
        .call("lean_blueprint", json!({"root": "mid_fact"}))
        .await;
    let names: Vec<_> = envelope.data["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["base_fact", "mid_fact"]);
    assert_eq!(envelope.data["root"], "mid_fact");
}

#[tokio::test]
async fn blueprint_carries_the_ledgers_claim_ids() {
    let stub = service().await;
    let mut cfg = config(Some(&stub.base));
    cfg.query_timeout_ms = 200;
    let dir = tempfile::tempdir().unwrap();
    let workspace = Arc::new(supermath_files::Workspace::new(dir.path()));
    let lean = Lean::with_claim_index(cfg, workspace.clone(), Arc::new(Claims)).unwrap();
    let write = |path: &str, text: &str| {
        let path = supermath_files::ProjectPath::parse(path).unwrap();
        workspace
            .write(
                "demo",
                &path,
                text,
                None,
                &supermath_files::WriteMeta::new("file_write", "c"),
            )
            .unwrap();
    };
    write("lean/Supermath/Base.lean", BASE);
    write("lean/Supermath/Mid.lean", MID);
    write("lean/Supermath/Top.lean", TOP);
    let tool = lean
        .tools()
        .into_iter()
        .find(|t| t.name() == "lean_blueprint")
        .unwrap();
    let envelope = tool.call(&ctx(), json!({})).await;
    assert_eq!(node(&envelope, "top_claim")["claim_id"], "claim-7");
    assert_eq!(node(&envelope, "mid_fact")["claim_id"], Value::Null);
}

#[tokio::test]
async fn blueprint_pages_and_marks_files_it_could_not_check() {
    let (_stub, env) = project().await;
    env.put(
        "lean/Supermath/Gone.lean",
        "import Mathlib\n\ntheorem gone_claim : True := trivial\n",
    );
    env.put(
        "lean/Supermath/Slow.lean",
        "import Mathlib\n\ntheorem slow_claim : True := trivial\n",
    );
    let first = env.call("lean_blueprint", json!({"limit": 2})).await;
    assert_eq!(first.data["nodes"].as_array().unwrap().len(), 2);
    assert!(first.cursor.is_some());
    assert!(first.unknowns.iter().any(|u| u.contains("Gone.lean")));
    assert!(first.unknowns.iter().any(|u| u.contains("Slow.lean")));
    let all = env.call("lean_blueprint", json!({})).await;
    assert_eq!(node(&all, "slow_claim")["status"], "unknown");
    assert!(
        all.data["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["name"] != "gone_claim")
    );
}

#[tokio::test]
async fn blueprint_marks_a_failed_files_declarations_failed_and_a_sorry_without_axioms() {
    let stub = Stub::start(|r| {
        let path = r.body["path"].as_str().unwrap_or("");
        if path.ends_with("Broken.lean") {
            support::Reply::fixture("check_broken.json")
        } else {
            support::Reply::json(&json!({"status": "ok", "toolchain": "t", "mathlib": "m",
                "messages": [], "sorries": [], "axioms": {}}))
        }
    })
    .await;
    let env = Env::at(&stub.base);
    env.put(
        "lean/Supermath/Broken.lean",
        "theorem broken_claim : False := by\n  rfl\n",
    );
    env.put(
        "lean/Supermath/Plain.lean",
        "theorem plain : True := trivial\ntheorem unfinished : True := by\n  sorry\n",
    );
    let envelope = env.call("lean_blueprint", json!({})).await;
    assert_eq!(node(&envelope, "broken_claim")["status"], "failed");
    assert_eq!(node(&envelope, "plain")["status"], "checked");
    assert_eq!(
        node(&envelope, "unfinished")["status"],
        "checked_with_sorry"
    );
}

#[tokio::test]
async fn blueprint_refuses_bad_roots_and_a_dead_service() {
    let (_stub, env) = project().await;
    let missing = env.call("lean_blueprint", json!({"root": "nope"})).await;
    assert_eq!(missing.code(), Some(ErrorCode::NotFound));
    let bad = env.call("lean_blueprint", json!({"root": "a b"})).await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
    let down = Env::new(config(Some(&dead_url().await)));
    down.put("lean/Supermath/Base.lean", BASE);
    assert_eq!(
        down.call("lean_blueprint", json!({})).await.code(),
        Some(ErrorCode::BackendUnavailable)
    );
}

#[tokio::test]
async fn lean_blueprint_passes_the_contract_checker() {
    let (_stub, env) = project().await;
    env.put_in("downproj", "lean/Supermath/Base.lean", BASE);
    let fixtures = [
        Fixture::with_ctx(ctx(), json!({"root": "mid_fact"})),
        Fixture::with_ctx(ctx(), json!({"root": "nope"})),
        Fixture::with_ctx(ctx(), json!({"root": "a b"})),
        Fixture::with_ctx(down_ctx(), json!({})),
    ];
    let tool: Arc<dyn Tool> = env.tool("lean_blueprint");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
}

// --- unreadable files, default page size, cursors -------------------------------

/// Puts a Lean file whose bytes are not UTF-8 straight onto disk: the
/// workspace lists it but cannot read it.
fn put_unreadable(env: &Env, path: &str) {
    let full = env.dir.path().join("demo").join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, [0xff, 0xfe, 0x00]).unwrap();
}

/// A project of `count` independent one-theorem files over a stub that
/// answers every check clean.
async fn many(count: usize) -> (Stub, Env) {
    let stub = Stub::start(|_| support::Reply::fixture("check_clean.json")).await;
    let env = Env::over(&stub);
    for i in 0..count {
        env.put(
            &format!("lean/Supermath/F{i:02}.lean"),
            &format!("import Mathlib\n\ntheorem f{i} : True := trivial\n"),
        );
    }
    (stub, env)
}

#[tokio::test]
async fn build_reports_an_unreadable_file_instead_of_skipping_it() {
    let (_stub, env) = project().await;
    put_unreadable(&env, "lean/Supermath/Bad.lean");
    let envelope = env.call("lean_build", json!({})).await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(envelope.data["total"], 4);
    assert_eq!(envelope.data["summary"]["unknown"], 1);
    let bad = files(&envelope)
        .into_iter()
        .find(|f| f["path"] == "lean/Supermath/Bad.lean")
        .unwrap();
    assert_eq!(bad["verdict"], "unknown");
    assert!(
        bad["reason"]
            .as_str()
            .unwrap()
            .contains("could not be read")
    );
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u.contains("1 Lean file(s) could not be read")
                && u.contains("lean/Supermath/Bad.lean")),
        "{:?}",
        envelope.unknowns
    );
}

#[tokio::test]
async fn build_pages_ten_rows_by_default_and_past_the_end_is_an_empty_page() {
    let (_stub, env) = many(12).await;
    let first = env.call("lean_build", json!({})).await;
    assert_eq!(files(&first).len(), 10);
    assert_eq!(first.cursor.as_deref(), Some("10"));
    let past = env.call("lean_build", json!({"cursor": "99"})).await;
    assert_eq!(past.outcome, Outcome::Ok);
    assert!(files(&past).is_empty());
    assert_eq!(past.data["total"], 12);
    assert!(past.cursor.is_none());
}

#[tokio::test]
async fn blueprint_lists_an_unreadable_file_in_unknowns() {
    let (_stub, env) = project().await;
    put_unreadable(&env, "lean/Supermath/Bad.lean");
    let envelope = env.call("lean_blueprint", json!({})).await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert!(
        envelope
            .unknowns
            .iter()
            .any(|u| u.contains("lean/Supermath/Bad.lean could not be read")),
        "{:?}",
        envelope.unknowns
    );
}

#[tokio::test]
async fn blueprint_pages_ten_nodes_by_default_and_refuses_a_bad_cursor() {
    let (_stub, env) = many(12).await;
    let first = env.call("lean_blueprint", json!({})).await;
    assert_eq!(first.data["nodes"].as_array().unwrap().len(), 10);
    assert_eq!(first.cursor.as_deref(), Some("10"));
    let bad = env.call("lean_blueprint", json!({"cursor": "x"})).await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
    let past = env.call("lean_blueprint", json!({"cursor": "99"})).await;
    assert_eq!(past.outcome, Outcome::Ok);
    assert!(past.data["nodes"].as_array().unwrap().is_empty());
    assert_eq!(past.data["total"], 12);
    assert!(past.cursor.is_none());
}
