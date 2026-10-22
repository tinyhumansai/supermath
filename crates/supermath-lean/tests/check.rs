//! `lean_check`, the verdict table, caching, and the checking API, through the
//! public API against a loopback stub replaying recorded service answers.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use serde_json::{Value, json};
use supermath_files::{FileObserver, ProjectPath};
use supermath_lean::Verdict;
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{Budget, Envelope, ErrorCode, Exposure, Outcome};
use support::{Env, Reply, Stub, check_fixture, config, ctx, dead_url};

const CLEAN: &str = "import Mathlib\n\ntheorem two_add_two : 2 + 2 = 4 := by\n  norm_num\n";
const SORRIED: &str = "import Mathlib\n\ntheorem main_claim : 1 + 1 = 2 := by\n  sorry\n";
const ADMITTED: &str = "import Mathlib\n\ntheorem admitted_claim : 1 + 1 = 2 := by\n  admit\n";
const BROKEN: &str = "import Mathlib\n\ntheorem broken_claim : 1 + 1 = 3 := by\n  rfl\n";
const CUSTOM: &str = "axiom my_axiom : False\n\ntheorem uses_it : False := my_axiom\n";
const NATIVE: &str = "import Mathlib\n\ntheorem big_fact : 2 ^ 64 > 0 := by\n  native_decide\n";
const SLOW: &str = "import Mathlib\n\ntheorem slow_claim : True := by\n  trivial\n";

async fn stub() -> Stub {
    Stub::start(|r| match r.path.as_str() {
        "/check" if r.body["path"].as_str().unwrap().ends_with("Down.lean") => {
            Reply::status(500, "the repl pool is restarting")
        }
        "/check" => check_fixture(r.body["path"].as_str().unwrap()),
        _ => Reply::status(404, "no such route"),
    })
    .await
}

async fn check(env: &Env, path: &str) -> Envelope {
    env.call("lean_check", json!({"path": path})).await
}

fn data(envelope: &Envelope) -> &Value {
    &envelope.data
}

// --- one test per row of the verdict table -----------------------------------

#[tokio::test]
async fn a_clean_proof_is_checked_with_full_provenance() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    let path = "lean/Supermath/Clean.lean";
    let sha = env.put(path, CLEAN);
    let envelope = check(&env, path).await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(data(&envelope)["verdict"], "checked");
    let provenance = &data(&envelope)["provenance"];
    assert_eq!(provenance["toolchain"], "leanprover/lean4:v4.20.0");
    assert_eq!(provenance["mathlib"], "4f2c9e1ab07d");
    assert_eq!(provenance["path"], path);
    assert_eq!(provenance["file_sha256"], sha);
    let imports = env
        .workspace
        .imports_digest("demo", &ProjectPath::parse(path).unwrap())
        .unwrap();
    assert_eq!(provenance["imports_sha256"], imports.sha256);
    assert_eq!(provenance["elapsed_ms"], 1840);
    assert_eq!(provenance["axioms"]["two_add_two"], json!(["propext"]));
    assert_eq!(provenance["debt"], json!([]));
    assert!(data(&envelope)["errors"].as_array().unwrap().is_empty());
    assert!(envelope.next.is_empty());
}

#[tokio::test]
async fn a_sorry_is_checked_with_sorry_and_points_at_the_goal() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Sorried.lean", SORRIED);
    let envelope = check(&env, "lean/Supermath/Sorried.lean").await;
    assert_eq!(data(&envelope)["verdict"], "checked_with_sorry");
    assert_eq!(data(&envelope)["sorries"][0]["goal"], "⊢ 1 + 1 = 2");
    assert_eq!(data(&envelope)["sorries"][0]["line"], 4);
    assert!(envelope.next[0].starts_with("lean_goals "));
    assert!(envelope.next[1].starts_with("lean_try_tactics "));
}

#[tokio::test]
async fn an_admit_is_checked_with_sorry() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Admitted.lean", ADMITTED);
    let envelope = check(&env, "lean/Supermath/Admitted.lean").await;
    assert_eq!(data(&envelope)["verdict"], "checked_with_sorry");
}

#[tokio::test]
async fn an_error_is_failed_and_listed() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Broken.lean", BROKEN);
    let envelope = check(&env, "lean/Supermath/Broken.lean").await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(data(&envelope)["verdict"], "failed");
    assert_eq!(data(&envelope)["error_count"], 1);
    let error = &data(&envelope)["errors"][0];
    assert_eq!(error["line"], 4);
    assert_eq!(error["column"], 2);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("unsolved goals")
    );
}

#[tokio::test]
async fn a_custom_axiom_is_failed_with_untrusted_axiom() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Custom.lean", CUSTOM);
    let envelope = check(&env, "lean/Supermath/Custom.lean").await;
    assert_eq!(data(&envelope)["verdict"], "failed");
    let codes: Vec<_> = data(&envelope)["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["code"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(codes, ["UNTRUSTED_AXIOM", "UNTRUSTED_AXIOM"]);
}

#[tokio::test]
async fn native_decide_is_checked_with_named_debt() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Native.lean", NATIVE);
    let envelope = check(&env, "lean/Supermath/Native.lean").await;
    assert_eq!(data(&envelope)["verdict"], "checked");
    assert_eq!(data(&envelope)["provenance"]["debt"], json!(["big_fact"]));
}

#[tokio::test]
async fn a_service_timeout_is_a_timeout_error_with_an_unknown_verdict() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Slow.lean", SLOW);
    let envelope = check(&env, "lean/Supermath/Slow.lean").await;
    assert_eq!(envelope.code(), Some(ErrorCode::Timeout));
    assert_eq!(data(&envelope)["verdict"], "unknown");
}

#[tokio::test]
async fn running_out_of_memory_is_unknown_not_an_error() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Hungry.lean", SLOW);
    let envelope = check(&env, "lean/Supermath/Hungry.lean").await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(data(&envelope)["verdict"], "unknown");
    assert!(envelope.unknowns[0].contains("memory"));
}

#[tokio::test]
async fn a_client_side_timeout_is_also_unknown_and_never_cached() {
    let stub = Stub::start(|_| Reply::fixture("check_clean.json").delayed(400)).await;
    let mut cfg = config(Some(&stub.base));
    cfg.check_timeout_ms = 50;
    let env = Env::new(cfg);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let envelope = check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(envelope.code(), Some(ErrorCode::Timeout));
    assert_eq!(data(&envelope)["verdict"], "unknown");
    assert_eq!(data(&envelope)["provenance"]["toolchain"], "");
    assert!(
        env.lean
            .observer()
            .last_result(
                "demo",
                &ProjectPath::parse("lean/Supermath/Clean.lean").unwrap()
            )
            .is_none()
    );
}

// --- the request, caching, and staleness --------------------------------------

#[tokio::test]
async fn the_request_names_the_project_path_targets_and_budget() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(
        stub.last_body("/check"),
        json!({"project": "demo", "path": "lean/Supermath/Clean.lean",
               "targets": ["two_add_two"], "uses": false, "timeout_ms": 60000})
    );
}

#[tokio::test]
async fn an_unchanged_file_reuses_its_verdict_and_says_so() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let first = check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(first.coverage.as_ref().unwrap().cached_age, None);
    let second = check(&env, "lean/Supermath/Clean.lean").await;
    assert!(second.coverage.as_ref().unwrap().cached_age.is_some());
    assert_eq!(data(&first), data(&second));
    assert_eq!(stub.count("/check"), 1);
}

#[tokio::test]
async fn a_changed_file_is_rechecked() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    check(&env, "lean/Supermath/Clean.lean").await;
    env.put(
        "lean/Supermath/Clean.lean",
        &format!("{CLEAN}\n-- edited\n"),
    );
    check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(stub.count("/check"), 2);
}

#[tokio::test]
async fn a_changed_import_makes_the_importer_stale() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Helper.lean", "import Mathlib\n");
    let importer = "import Supermath.Helper\n\ntheorem two_add_two : 2 + 2 = 4 := by\n  norm_num\n";
    env.put("lean/Supermath/Clean.lean", importer);
    check(&env, "lean/Supermath/Clean.lean").await;
    let path = ProjectPath::parse("lean/Supermath/Clean.lean").unwrap();
    let observer = env.lean.observer();
    let fresh = observer.last_result("demo", &path).unwrap();
    assert_eq!(fresh.sha256, env.sha("lean/Supermath/Clean.lean"));

    env.put("lean/Supermath/Helper.lean", "import Mathlib\n-- changed\n");
    let stale = observer.last_result("demo", &path).unwrap();
    assert_ne!(stale.sha256, env.sha("lean/Supermath/Clean.lean"));
    check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(stub.count("/check"), 2);
}

#[tokio::test]
async fn different_targets_are_not_served_from_another_targets_cache() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    check(&env, "lean/Supermath/Clean.lean").await;
    let named = env
        .call(
            "lean_check",
            json!({"path": "lean/Supermath/Clean.lean", "targets": ["two_add_two"]}),
        )
        .await;
    assert_eq!(data(&named)["verdict"], "checked");
    assert_eq!(stub.count("/check"), 1, "same targets, so cached");
    let other = env
        .call(
            "lean_check",
            json!({"path": "lean/Supermath/Clean.lean", "targets": ["two_add_two", "extra"]}),
        )
        .await;
    assert_eq!(other.code(), Some(ErrorCode::NotFound));
    assert_eq!(stub.count("/check"), 2);
}

#[tokio::test]
async fn the_observer_reports_the_last_verdict_and_its_digest() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    let sha = env.put("lean/Supermath/Sorried.lean", SORRIED);
    let path = ProjectPath::parse("lean/Supermath/Sorried.lean").unwrap();
    let observer = env.lean.observer();
    assert!(observer.last_result("demo", &path).is_none());
    check(&env, "lean/Supermath/Sorried.lean").await;
    let last = observer.last_result("demo", &path).unwrap();
    assert_eq!(last.result, "checked_with_sorry");
    assert_eq!(last.sha256, sha);
    let new_sha = env.put(
        "lean/Supermath/Sorried.lean",
        &format!("{SORRIED}-- more\n"),
    );
    let last = observer.last_result("demo", &path).unwrap();
    assert_ne!(
        last.sha256, new_sha,
        "the old verdict is stale for the new file"
    );
}

// --- errors --------------------------------------------------------------------

#[tokio::test]
async fn a_missing_file_a_missing_target_and_bad_paths_are_refused() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    let missing = check(&env, "lean/Supermath/Nowhere.lean").await;
    assert_eq!(missing.code(), Some(ErrorCode::NotFound));
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let target = env
        .call(
            "lean_check",
            json!({"path": "lean/Supermath/Clean.lean", "targets": ["nope"]}),
        )
        .await;
    assert_eq!(target.code(), Some(ErrorCode::NotFound));
    for args in [
        json!({"path": "lean/../x.lean"}),
        json!({"path": "python/work/a.py"}),
        json!({"path": "lean/Supermath/Clean.lean", "targets": []}),
        json!({"path": "lean/Supermath/Clean.lean", "targets": ["a b"]}),
        json!({"path": "lean/Supermath/Clean.lean", "source": "theorem x"}),
    ] {
        let envelope = env.call("lean_check", args.clone()).await;
        assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput), "{args}");
    }
    assert_eq!(
        stub.count("/check"),
        1,
        "only the unknown target reached the service"
    );
}

#[tokio::test]
async fn source_over_the_budget_is_refused_before_the_service_is_called() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let mut context = ctx();
    context.budget = Budget {
        bytes: 10,
        ..Budget::default()
    };
    let envelope = env
        .tool("lean_check")
        .call(&context, json!({"path": "lean/Supermath/Clean.lean"}))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::BudgetExceeded));
    assert_eq!(stub.count("/check"), 0);
}

#[tokio::test]
async fn with_no_service_every_check_is_backend_unavailable() {
    let env = Env::new(config(None));
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let envelope = check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn a_service_that_is_down_is_backend_unavailable() {
    let env = Env::new(config(Some(&dead_url().await)));
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let envelope = check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn a_service_without_the_file_is_not_found() {
    let stub = Stub::start(|_| Reply::status(404, "no such file on the projects volume")).await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let envelope = check(&env, "lean/Supermath/Clean.lean").await;
    assert_eq!(envelope.code(), Some(ErrorCode::NotFound));
}

// --- the checking API for the ledger -------------------------------------------

#[tokio::test]
async fn verify_returns_a_report_and_is_current_tracks_the_digests() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let path = ProjectPath::parse("lean/Supermath/Clean.lean").unwrap();
    let checker = env.lean.checker();
    let report = checker
        .verify("demo", &path, "two_add_two", 60_000)
        .await
        .unwrap();
    assert_eq!(report.verdict, Verdict::Checked);
    assert_eq!(report.targets, ["two_add_two"]);
    assert!(checker.is_current("demo", &report.provenance).unwrap());
    env.put("lean/Supermath/Clean.lean", &format!("{CLEAN}-- edit\n"));
    assert!(!checker.is_current("demo", &report.provenance).unwrap());

    let mut gone = report.provenance.clone();
    gone.path = "lean/Supermath/Gone.lean".into();
    assert!(checker.is_current("demo", &gone).is_err());
    gone.path = "nonsense".into();
    assert!(checker.is_current("demo", &gone).is_err());
}

#[tokio::test]
async fn verify_rejects_python_files_and_unknown_declarations() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    let checker = env.lean.checker();
    let py = ProjectPath::parse("python/work/a.py").unwrap();
    assert!(checker.verify("demo", &py, "x", 1_000).await.is_err());
    let lean = ProjectPath::parse("lean/Supermath/Clean.lean").unwrap();
    assert!(
        checker
            .verify("demo", &lean, "absent", 1_000)
            .await
            .is_err()
    );
}

// --- the contract ----------------------------------------------------------------

#[tokio::test]
async fn lean_check_passes_the_contract_checker() {
    let stub = stub().await;
    let env = Env::at(&stub.base);
    env.put("lean/Supermath/Clean.lean", CLEAN);
    env.put("lean/Supermath/Slow.lean", SLOW);
    env.put("lean/Supermath/Down.lean", SLOW);
    let mut tiny = ctx();
    tiny.budget.bytes = 10;
    let fixtures = [
        Fixture::with_ctx(ctx(), json!({"path": "lean/Supermath/Clean.lean"})),
        Fixture::with_ctx(ctx(), json!({"path": "lean/Supermath/Slow.lean"})),
        Fixture::with_ctx(ctx(), json!({"path": "lean/Supermath/Down.lean"})),
        Fixture::with_ctx(ctx(), json!({"path": "lean/Supermath/Nowhere.lean"})),
        Fixture::with_ctx(ctx(), json!({})),
        Fixture::with_ctx(tiny, json!({"path": "lean/Supermath/Clean.lean"})),
    ];
    let tool = env.tool("lean_check");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
    assert_eq!(tool.exposure(), Exposure::Direct);
}
