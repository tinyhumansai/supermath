//! `lean_search_counterexample`: finite exhaustive search, the generated
//! witness file, and what is recorded about a search that finds nothing.

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
use support::{Env, Reply, Stub, config, ctx, service};

const SEARCH: &str = "import Mathlib\n\nnamespace Euler\n\ndef notPrime (n : ℕ) : Prop := 1 < n ∧ ¬ n.Prime\n\nend Euler\n\ndef noneSuch (n : ℕ) : Prop := False\ndef tooSlow (n : ℕ) : Prop := False\ndef tooSlowEarly (n : ℕ) : Prop := False\ndef notDecidable (n : ℕ) : Prop := True\ndef badWitness (n : ℕ) : Prop := True\ndef negCheck (z : ℤ) : Prop := z < 0\ndef weird (n : ℕ) : Prop := True\n";
const SEARCH_PATH: &str = "lean/Supermath/Euler/Search.lean";
const WITNESS_PATH: &str = "lean/Supermath/Witnesses/EulerNotPrime.lean";

fn args(predicate: &str, from: i64, to: i64) -> Value {
    json!({"path": SEARCH_PATH, "predicate": predicate, "range": {"from": from, "to": to}})
}

async fn setup() -> (Stub, Env) {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put(SEARCH_PATH, SEARCH);
    (stub, env)
}

fn read(env: &Env, path: &str) -> String {
    let path = supermath_files::ProjectPath::parse(path).unwrap();
    env.workspace.read("demo", &path).unwrap().text
}

#[tokio::test]
async fn a_witness_is_recorded_as_a_checked_theorem() {
    let (stub, env) = setup().await;
    let envelope = env
        .call("lean_search_counterexample", args("Euler.notPrime", 0, 100))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(envelope.data["found"], true);
    assert_eq!(envelope.data["witness"], 91);
    assert_eq!(envelope.data["verdict"], "checked");
    assert_eq!(envelope.data["witness_file"], WITNESS_PATH);
    assert_eq!(envelope.data["theorem"], "Euler.notPrime_witness");
    assert_eq!(envelope.data["evidence_origin"], "collected");
    assert_eq!(
        envelope.data["frame"],
        json!({"predicate": "Euler.notPrime", "from": 0, "to": 100, "covered_to": 91, "domain": "nat"})
    );
    assert_eq!(envelope.data["provenance"]["path"], WITNESS_PATH);
    assert_eq!(
        read(&env, WITNESS_PATH),
        "import Supermath.Euler.Search\n\ntheorem Euler.notPrime_witness : ∃ n : ℕ, Euler.notPrime n := ⟨91, by decide⟩\n"
    );
    assert_eq!(
        stub.last_body("/counterexample"),
        json!({"project": "demo", "path": SEARCH_PATH, "predicate": "Euler.notPrime",
               "from": 0, "to": 100, "timeout_ms": 60000})
    );
    // The checked theorem named the generated file's write in the log.
    let log = env.workspace.write_log("demo").unwrap();
    let writes = log.for_call("call-1").unwrap();
    assert_eq!(writes[0].path, WITNESS_PATH);
    assert_eq!(writes[0].tool, "lean_search_counterexample");
}

#[tokio::test]
async fn searching_again_leaves_an_identical_witness_file_alone() {
    let (_stub, env) = setup().await;
    env.call("lean_search_counterexample", args("Euler.notPrime", 0, 100))
        .await;
    let first = env.sha(WITNESS_PATH);
    let again = env
        .call("lean_search_counterexample", args("Euler.notPrime", 0, 100))
        .await;
    assert_eq!(again.outcome, Outcome::Ok);
    assert_eq!(env.sha(WITNESS_PATH), first);
}

#[tokio::test]
async fn a_changed_witness_replaces_the_old_generated_file() {
    let (_stub, env) = setup().await;
    env.put(WITNESS_PATH, "import Supermath.Euler.Search\n-- stale\n");
    let envelope = env
        .call("lean_search_counterexample", args("Euler.notPrime", 0, 100))
        .await;
    assert_eq!(envelope.data["found"], true);
    assert!(read(&env, WITNESS_PATH).contains("⟨91, by decide⟩"));
}

#[tokio::test]
async fn an_integer_witness_is_ascribed_in_the_theorem() {
    let (_stub, env) = setup().await;
    let envelope = env
        .call("lean_search_counterexample", args("negCheck", -10, 10))
        .await;
    assert_eq!(envelope.data["witness"], -3);
    assert!(envelope.coverage.unwrap().searched[0].contains('ℤ'));
    assert!(read(&env, "lean/Supermath/Witnesses/NegCheck.lean").contains("⟨(-3 : ℤ), by decide⟩"));
}

#[tokio::test]
async fn a_witness_that_does_not_check_is_not_evidence() {
    let (_stub, env) = setup().await;
    let envelope = env
        .call("lean_search_counterexample", args("badWitness", 0, 10))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(envelope.data["verdict"], "failed");
    assert!(envelope.unknowns[0].contains("not evidence"));
}

#[tokio::test]
async fn a_fully_covered_range_with_no_witness_is_ok_exhausted_never_false() {
    let (_stub, env) = setup().await;
    let envelope = env
        .call("lean_search_counterexample", args("noneSuch", 0, 100))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(envelope.data["result"], "exhausted");
    assert_eq!(envelope.data["found"], false);
    assert_eq!(envelope.data["frame"]["covered_to"], 100);
    assert_eq!(envelope.data["frame"]["from"], 0);
    assert_eq!(envelope.unknowns.len(), 0);
    assert!(envelope.coverage.unwrap().not_covered[0].contains("outside the range"));
    let path =
        supermath_files::ProjectPath::parse("lean/Supermath/Witnesses/NoneSuch.lean").unwrap();
    assert!(!env.workspace.exists("demo", &path).unwrap());
}

#[tokio::test]
async fn a_budget_hit_reports_how_far_the_search_got() {
    let (_stub, env) = setup().await;
    let envelope = env
        .call("lean_search_counterexample", args("tooSlow", 0, 100_000))
        .await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(envelope.data["frame"]["covered_to"], 4000);
    assert!(envelope.unknowns[0].contains("0..=4000"));
    let early = env
        .call(
            "lean_search_counterexample",
            args("tooSlowEarly", 0, 100_000),
        )
        .await;
    assert!(early.unknowns[0].contains("before covering any value"));
    assert_eq!(early.data["frame"]["covered_to"], Value::Null);
}

#[tokio::test]
async fn a_slow_service_is_unknown_not_an_error() {
    let stub = Stub::start(|_| Reply::fixture("counter_found.json").delayed(400)).await;
    let mut cfg = config(Some(&stub.base));
    cfg.check_timeout_ms = 50;
    let env = Env::new(cfg);
    env.put(SEARCH_PATH, SEARCH);
    let envelope = env
        .call("lean_search_counterexample", args("noneSuch", 0, 10))
        .await;
    assert_eq!(envelope.outcome, Outcome::Unknown);
    assert_eq!(envelope.data["found"], false);
}

#[tokio::test]
async fn a_predicate_that_is_not_decidable_is_invalid_input_with_leans_error() {
    let (_stub, env) = setup().await;
    let envelope = env
        .call("lean_search_counterexample", args("notDecidable", 0, 10))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput));
    assert!(envelope.error.unwrap().message.contains("DecidablePred"));
}

#[tokio::test]
async fn an_unknown_service_status_is_backend_unavailable() {
    let (_stub, env) = setup().await;
    let envelope = env
        .call("lean_search_counterexample", args("weird", 0, 10))
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn bad_requests_are_refused_before_the_service_is_called() {
    let (stub, env) = setup().await;
    let refused =
        |envelope: &supermath_tool::Envelope, code| assert_eq!(envelope.code(), Some(code));
    refused(
        &env.call("lean_search_counterexample", args("noneSuch", 5, 4))
            .await,
        ErrorCode::InvalidInput,
    );
    refused(
        &env.call("lean_search_counterexample", args("noneSuch", 0, 1_000_000))
            .await,
        ErrorCode::BudgetExceeded,
    );
    refused(
        &env.call(
            "lean_search_counterexample",
            args("noneSuch", i64::MIN, i64::MAX),
        )
        .await,
        ErrorCode::BudgetExceeded,
    );
    refused(
        &env.call("lean_search_counterexample", args("no such", 0, 4))
            .await,
        ErrorCode::InvalidInput,
    );
    refused(
        &env.call("lean_search_counterexample", args("Euler.missing", 0, 4))
            .await,
        ErrorCode::NotFound,
    );
    let mut other = args("noneSuch", 0, 4);
    other["path"] = json!("lean/Supermath/Nowhere.lean");
    refused(
        &env.call("lean_search_counterexample", other).await,
        ErrorCode::NotFound,
    );
    let mut python = args("noneSuch", 0, 4);
    python["path"] = json!("python/work/a.py");
    refused(
        &env.call("lean_search_counterexample", python).await,
        ErrorCode::InvalidInput,
    );
    refused(
        &env.call("lean_search_counterexample", json!({"path": SEARCH_PATH}))
            .await,
        ErrorCode::InvalidInput,
    );
    assert_eq!(stub.count("/counterexample"), 0);
    // The cap is inclusive of exactly one million values.
    let exact = env
        .call("lean_search_counterexample", args("noneSuch", 0, 999_999))
        .await;
    assert_eq!(exact.outcome, Outcome::Ok);
}

#[tokio::test]
async fn lean_search_counterexample_passes_the_contract_checker() {
    let (_stub, env) = setup().await;
    env.put("lean/Supermath/Down.lean", SEARCH);
    let mut down = args("noneSuch", 0, 10);
    down["path"] = json!("lean/Supermath/Down.lean");
    let fixtures = [
        Fixture::with_ctx(ctx(), args("Euler.notPrime", 0, 100)),
        Fixture::with_ctx(ctx(), args("noneSuch", 0, 100)),
        Fixture::with_ctx(ctx(), args("notDecidable", 0, 10)),
        Fixture::with_ctx(ctx(), args("noneSuch", 0, 2_000_000)),
        Fixture::with_ctx(ctx(), args("Euler.missing", 0, 10)),
        Fixture::with_ctx(ctx(), down),
    ];
    let tool = env.tool("lean_search_counterexample");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
}
