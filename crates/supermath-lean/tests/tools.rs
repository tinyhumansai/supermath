//! `lean_goals`, `lean_try_tactics`, and `lean_decl` through the public API
//! against the recorded service answers.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

mod support;

use serde_json::{Value, json};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{CallContext, ErrorCode, Exposure, Outcome};
use support::{Env, ctx, service};

const SORRIED: &str = "import Mathlib\n\ntheorem main_claim : 1 + 1 = 2 := by\n  sorry\n";

fn tiny() -> CallContext {
    let mut context = ctx();
    context.budget.bytes = 10;
    context
}

fn fixture(args: Value) -> Fixture {
    Fixture::with_ctx(ctx(), args)
}

// --- lean_goals -------------------------------------------------------------------

#[tokio::test]
async fn goals_returns_the_pretty_printed_state_with_the_file_digest() {
    let stub = service().await;
    let env = Env::over(&stub);
    let sha = env.put("lean/Supermath/Main.lean", SORRIED);
    let envelope = env
        .call(
            "lean_goals",
            json!({"path": "lean/Supermath/Main.lean", "line": 4, "column": 2}),
        )
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(envelope.data["goals"], "a b : ℕ\n⊢ a + b = b + a");
    assert_eq!(envelope.data["sha256"], sha);
    assert_eq!(envelope.data["truncated"], false);
    assert_eq!(
        stub.last_body("/goals"),
        json!({"project": "demo", "path": "lean/Supermath/Main.lean",
               "line": 4, "column": 2, "timeout_ms": 60000})
    );
}

#[tokio::test]
async fn goals_are_cut_at_eight_thousand_characters() {
    let long = "x".repeat(9_000);
    let stub = support::Stub::start(move |_| {
        support::Reply::json(&json!({"status": "ok", "goals": long}))
    })
    .await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Main.lean", SORRIED);
    let envelope = env
        .call(
            "lean_goals",
            json!({"path": "lean/Supermath/Main.lean", "line": 1, "column": 0}),
        )
        .await;
    assert_eq!(envelope.data["truncated"], true);
    assert_eq!(
        envelope.data["goals"].as_str().unwrap().chars().count(),
        8_001
    );
}

#[tokio::test]
async fn goals_refuses_positions_outside_the_file_without_asking_the_service() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Main.lean", SORRIED);
    for (line, column) in [(0, 0), (99, 0), (1, 500)] {
        let envelope = env
            .call(
                "lean_goals",
                json!({"path": "lean/Supermath/Main.lean", "line": line, "column": column}),
            )
            .await;
        assert_eq!(
            envelope.code(),
            Some(ErrorCode::InvalidInput),
            "{line}:{column}"
        );
    }
    assert_eq!(stub.count("/goals"), 0);
}

#[tokio::test]
async fn goals_passes_through_the_services_position_refusal() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Badpos.lean", SORRIED);
    let envelope = env
        .call(
            "lean_goals",
            json!({"path": "lean/Supermath/Badpos.lean", "line": 1, "column": 0}),
        )
        .await;
    assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput));
}

#[tokio::test]
async fn goals_passes_the_contract_checker() {
    let stub = service().await;
    let env = Env::over(&stub);
    for name in ["Main", "Slow", "Down"] {
        env.put(&format!("lean/Supermath/{name}.lean"), SORRIED);
    }
    let at =
        |name: &str| json!({"path": format!("lean/Supermath/{name}.lean"), "line": 1, "column": 0});
    let fixtures = [
        fixture(at("Main")),
        fixture(at("Slow")),
        fixture(at("Down")),
        fixture(at("Nowhere")),
        fixture(json!({"path": "lean/Supermath/Main.lean", "line": 99, "column": 0})),
        Fixture::with_ctx(tiny(), at("Main")),
    ];
    let tool = env.tool("lean_goals");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
    assert_eq!(tool.exposure(), Exposure::Deferred);
}

// --- lean_try_tactics ---------------------------------------------------------------

fn tactics_args(path: &str, tactics: &Value) -> Value {
    json!({"path": path, "sorry": {"line": 4, "column": 2}, "tactics": tactics})
}

#[tokio::test]
async fn try_tactics_reports_each_outcome_and_builds_the_edit_for_the_first_closer() {
    let stub = service().await;
    let env = Env::over(&stub);
    let sha = env.put("lean/Supermath/Main.lean", SORRIED);
    let envelope = env
        .call(
            "lean_try_tactics",
            tactics_args("lean/Supermath/Main.lean", &json!(["rfl", "omega", "simp"])),
        )
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    let results = envelope.data["results"].as_array().unwrap();
    assert_eq!(results[0]["result"], "fails");
    assert!(results[0]["error"].as_str().unwrap().contains("rfl"));
    assert_eq!(results[1]["result"], "closes");
    assert_eq!(results[2]["result"], "progress");
    assert_eq!(results[2]["goals"], json!(["⊢ b + a = b + a"]));
    let call = envelope.next[0].strip_prefix("file_edit ").unwrap();
    let edit: Value = serde_json::from_str(call).unwrap();
    assert_eq!(edit["path"], "lean/Supermath/Main.lean");
    assert_eq!(edit["if_sha256"], sha);
    assert_eq!(
        edit["edits"][0],
        json!({"old": "  sorry", "new": "  omega"})
    );
    // The file itself is untouched.
    assert_eq!(env.sha("lean/Supermath/Main.lean"), sha);
    let sent = stub.last_body("/tactics");
    assert_eq!(sent["tactics"], json!(["rfl", "omega", "simp"]));
    assert_eq!(sent["timeout_ms"], 10_000);
}

#[tokio::test]
async fn try_tactics_prefers_the_term_exact_found() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Exact.lean", SORRIED);
    let envelope = env
        .call(
            "lean_try_tactics",
            tactics_args("lean/Supermath/Exact.lean", &json!(["exact?"])),
        )
        .await;
    assert!(envelope.next[0].contains("Nat.add_comm a b"));
}

#[tokio::test]
async fn try_tactics_defaults_to_ten_tactics() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Main.lean", SORRIED);
    env.call(
        "lean_try_tactics",
        json!({"path": "lean/Supermath/Main.lean", "sorry": {"line": 4, "column": 2}}),
    )
    .await;
    let sent = stub.last_body("/tactics");
    assert_eq!(sent["tactics"].as_array().unwrap().len(), 10);
    assert_eq!(sent["tactics"][9], "exact?");
}

#[tokio::test]
async fn try_tactics_notes_when_no_ready_edit_can_be_built() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put(
        "lean/Supermath/Main.lean",
        "import Mathlib\n\ntheorem a : True := by\n  sorry\n\ntheorem b : True := by\n  sorry\n",
    );
    let envelope = env
        .call(
            "lean_try_tactics",
            tactics_args("lean/Supermath/Main.lean", &json!(["omega"])),
        )
        .await;
    assert!(envelope.next.is_empty());
    assert!(envelope.unknowns[0].contains("not unique"));
}

#[tokio::test]
async fn try_tactics_turns_an_unrecognised_outcome_into_a_failure() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Odd.lean", SORRIED);
    let envelope = env
        .call(
            "lean_try_tactics",
            tactics_args("lean/Supermath/Odd.lean", &json!(["magic"])),
        )
        .await;
    assert_eq!(envelope.data["results"][0]["result"], "fails");
}

#[tokio::test]
async fn try_tactics_checks_the_position_against_the_last_check_when_it_is_current() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Sorried.lean", SORRIED);
    env.call("lean_check", json!({"path": "lean/Supermath/Sorried.lean"}))
        .await;
    let wrong = env
        .call(
            "lean_try_tactics",
            json!({"path": "lean/Supermath/Sorried.lean", "sorry": {"line": 9, "column": 9}, "tactics": ["simp"]}),
        )
        .await;
    assert_eq!(wrong.code(), Some(ErrorCode::InvalidInput));
    assert_eq!(stub.count("/tactics"), 0);
    let right = env
        .call(
            "lean_try_tactics",
            tactics_args("lean/Supermath/Sorried.lean", &json!(["simp"])),
        )
        .await;
    assert_eq!(right.outcome, Outcome::Ok);
    // After an edit the old check says nothing; the service decides.
    env.put(
        "lean/Supermath/Sorried.lean",
        &format!("{SORRIED}-- edit\n"),
    );
    let after = env
        .call(
            "lean_try_tactics",
            json!({"path": "lean/Supermath/Sorried.lean", "sorry": {"line": 9, "column": 9}, "tactics": ["simp"]}),
        )
        .await;
    assert_eq!(after.outcome, Outcome::Ok);
}

#[tokio::test]
async fn try_tactics_refuses_a_stale_digest_with_conflict() {
    let stub = service().await;
    let env = Env::over(&stub);
    let sha = env.put("lean/Supermath/Main.lean", SORRIED);
    env.put("lean/Supermath/Main.lean", &format!("{SORRIED}-- edit\n"));
    let mut args = tactics_args("lean/Supermath/Main.lean", &json!(["simp"]));
    args["if_sha256"] = json!(sha);
    let envelope = env.call("lean_try_tactics", args).await;
    assert_eq!(envelope.code(), Some(ErrorCode::Conflict));
    assert_eq!(stub.count("/tactics"), 0);
    let mut fresh = tactics_args("lean/Supermath/Main.lean", &json!(["simp"]));
    fresh["if_sha256"] = json!(env.sha("lean/Supermath/Main.lean").to_uppercase());
    assert_eq!(
        env.call("lean_try_tactics", fresh).await.outcome,
        Outcome::Ok
    );
}

#[tokio::test]
async fn try_tactics_refuses_bad_tactic_lists() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Main.lean", SORRIED);
    let thirteen: Vec<String> = (0..13).map(|i| format!("simp [h{i}]")).collect();
    for tactics in [
        json!([]),
        json!(["  "]),
        json!(thirteen),
        json!(["x".repeat(501)]),
    ] {
        let envelope = env
            .call(
                "lean_try_tactics",
                tactics_args("lean/Supermath/Main.lean", &tactics),
            )
            .await;
        assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput), "{tactics}");
    }
    let zero = env
        .call(
            "lean_try_tactics",
            json!({"path": "lean/Supermath/Main.lean", "sorry": {"line": 0, "column": 0}}),
        )
        .await;
    assert_eq!(zero.code(), Some(ErrorCode::InvalidInput));
    assert_eq!(stub.count("/tactics"), 0);
}

#[tokio::test]
async fn try_tactics_passes_the_contract_checker() {
    let stub = service().await;
    let env = Env::over(&stub);
    for name in ["Main", "Slow", "Down", "Nosorry"] {
        env.put(&format!("lean/Supermath/{name}.lean"), SORRIED);
    }
    let stale = env.sha("lean/Supermath/Main.lean");
    let at = |name: &str| tactics_args(&format!("lean/Supermath/{name}.lean"), &json!(["simp"]));
    let mut conflict = at("Main");
    conflict["if_sha256"] = json!(format!("{stale}00"));
    let fixtures = [
        fixture(at("Main")),
        fixture(at("Slow")),
        fixture(at("Down")),
        fixture(at("Nosorry")),
        fixture(at("Nowhere")),
        fixture(conflict),
        Fixture::with_ctx(tiny(), at("Main")),
    ];
    let tool = env.tool("lean_try_tactics");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
}

// --- lean_decl ------------------------------------------------------------------------

#[tokio::test]
async fn decl_returns_the_signature_without_the_proof() {
    let stub = service().await;
    let env = Env::over(&stub);
    let envelope = env
        .call("lean_decl", json!({"name": "Nat.Prime.two_le"}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    assert_eq!(envelope.data["kind"], "theorem");
    assert_eq!(envelope.data["module"], "Mathlib.Data.Nat.Prime.Defs");
    assert_eq!(envelope.data["doc"], "A prime is at least two.");
    assert!(
        envelope.data["signature"]
            .as_str()
            .unwrap()
            .contains("2 ≤ p")
    );
    assert!(envelope.data.get("path").is_none());
    assert_eq!(stub.last_body("/decl")["path"], Value::Null);
}

#[tokio::test]
async fn decl_finds_a_project_declaration_and_adds_its_path_and_line() {
    let stub = service().await;
    let env = Env::over(&stub);
    env.put("lean/Supermath/Other.lean", "def unrelated := 1\n");
    env.put("python/work/a.py", "import math\n");
    env.put(
        "lean/Supermath/Sendov/Main.lean",
        "import Mathlib\n\nnamespace Sendov\n\ntheorem main : True := trivial\n\nend Sendov\n",
    );
    let envelope = env.call("lean_decl", json!({"name": "Sendov.main"})).await;
    assert_eq!(envelope.data["path"], "lean/Supermath/Sendov/Main.lean");
    assert_eq!(envelope.data["line"], 5);
    assert_eq!(envelope.data["module"], "Supermath.Sendov.Main");
    assert_eq!(
        stub.last_body("/decl")["path"],
        "lean/Supermath/Sendov/Main.lean"
    );
}

#[tokio::test]
async fn decl_refuses_names_that_are_not_names() {
    let stub = service().await;
    let env = Env::over(&stub);
    for name in ["", "a b", "x; #eval 1", "Nat.", ".Nat"] {
        let envelope = env.call("lean_decl", json!({"name": name})).await;
        assert_eq!(envelope.code(), Some(ErrorCode::InvalidInput), "{name:?}");
    }
    assert_eq!(stub.count("/decl"), 0);
}

#[tokio::test]
async fn decl_passes_the_contract_checker() {
    let stub = service().await;
    let env = Env::over(&stub);
    let named = |name: &str| fixture(json!({"name": name}));
    let fixtures = [
        named("Nat.Prime.two_le"),
        named("Missing.thing"),
        named("Slow.thing"),
        named("Down.thing"),
        named("not a name"),
    ];
    let tool = env.tool("lean_decl");
    contract::check(tool.as_ref(), "lean", &fixtures)
        .await
        .unwrap();
}
