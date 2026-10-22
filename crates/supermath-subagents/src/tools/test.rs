//! Tests for the four `agent_*` tools, against scripted children and fake
//! primitives. Each tool has a case where the scripted child claims success
//! and the fake Lean says otherwise; the tool must report the fake's verdict.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use serde_json::{Value, json};
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{Budget, CallContext, Coverage, Envelope, ErrorCode, Outcome, Project};

use crate::Subagents;
use crate::belt::Belt;
use crate::child::{ChildEnd, ChildKind};
use crate::fake::{ScriptRunner, World, ctx, script, subagents, tool, turn, world::digest};
use crate::limits::Limits;

const PATH: &str = "lean/Supermath/Main.lean";
const SORRY_FILE: &str = "import Mathlib\n\ntheorem main_thm : 1 + 1 = 2 := by\n  sorry\n";
const WITNESS: &str = "lean/Supermath/Witnesses/Notp.lean";
const WITNESS_FILE: &str =
    "import Mathlib\n\ntheorem notp_witness : ∃ n, ¬ P n := ⟨7, by decide⟩\n";

async fn call(built: &Subagents, name: &str, args: Value) -> Envelope {
    tool(built, name).call(&ctx(), args).await
}

fn setup(runner: &Arc<ScriptRunner>) -> (World, Subagents) {
    let world = World::new();
    let built = subagents(&world.registry(), runner);
    (world, built)
}

async fn write(belt: &Belt, path: &str, content: &str) {
    let answer = belt
        .call("file_write", json!({"path": path, "content": content}))
        .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
}

fn unknowns(envelope: &Envelope) -> String {
    envelope.unknowns.join(" | ")
}

fn other_project() -> CallContext {
    CallContext::new(
        Project::new("other", "ws:test/project:other").unwrap(),
        "call-2",
    )
    .with_budget(ctx().budget)
}

// ---------------------------------------------------------------- shared

#[tokio::test]
async fn bad_arguments_are_invalid_input_with_usage_on_every_tool() {
    let runner = ScriptRunner::new(script(|_, _| async { Ok(turn("never")) }));
    let (_, built) = setup(&runner);
    for (name, args) in [
        (
            "agent_digest_paper",
            json!({"arxiv_id": "2012.04125", "extra": 1}),
        ),
        ("agent_formalize", json!({"source": {"statement": "x"}})),
        ("agent_review_alignment", json!({"claim_id": 7})),
        ("agent_refute", json!({})),
    ] {
        let answer = call(&built, name, args).await;
        assert_eq!(answer.code(), Some(ErrorCode::InvalidInput), "{name}");
        assert_eq!(answer.usage.as_ref().unwrap().model, "scripted", "{name}");
    }
    assert_eq!(runner.scopes().len(), 0, "no child ran");
}

#[tokio::test]
async fn a_call_in_another_project_is_forbidden_and_runs_no_child() {
    let runner = ScriptRunner::new(script(|_, _| async { Ok(turn("never")) }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &["thm:main"]);
    let claim = world.add_claim(json!({"kind": "result", "statement": "s"}));
    for (name, args) in [
        ("agent_digest_paper", json!({"arxiv_id": "2012.04125"})),
        (
            "agent_formalize",
            json!({"source": {"statement": "x"}, "path": PATH}),
        ),
        ("agent_review_alignment", json!({"claim_id": claim})),
        ("agent_refute", json!({"claim_id": claim})),
    ] {
        let answer = tool(&built, name).call(&other_project(), args).await;
        assert_eq!(answer.code(), Some(ErrorCode::Forbidden), "{name}");
    }
    assert_eq!(runner.scopes().len(), 0);
}

#[tokio::test]
async fn a_budget_the_caller_cannot_afford_is_refused_before_the_child_runs() {
    let runner = ScriptRunner::new(script(|_, _| async { Ok(turn("never")) }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &[]);
    let claim = world.add_claim(json!({"kind": "result", "statement": "s"}));
    let poor = ctx().with_budget(Budget::agentic(1, 1));
    for (name, args) in [
        ("agent_digest_paper", json!({"arxiv_id": "2012.04125"})),
        (
            "agent_formalize",
            json!({"source": {"statement": "x"}, "path": PATH}),
        ),
        ("agent_review_alignment", json!({"claim_id": claim})),
        ("agent_refute", json!({"claim_id": claim})),
    ] {
        let answer = tool(&built, name).call(&poor, args).await;
        assert_eq!(answer.code(), Some(ErrorCode::BudgetExceeded), "{name}");
    }
    assert_eq!(runner.scopes().len(), 0);
}

#[tokio::test]
async fn a_budget_argument_above_the_maximum_or_zero_is_invalid_input() {
    let runner = ScriptRunner::new(script(|_, _| async { Ok(turn("never")) }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &[]);
    for budget in [json!({"wall_secs": 1801}), json!({"max_iterations": 0})] {
        let answer = call(
            &built,
            "agent_digest_paper",
            json!({"arxiv_id": "2012.04125", "budget": budget}),
        )
        .await;
        assert_eq!(answer.code(), Some(ErrorCode::InvalidInput));
    }
}

#[tokio::test]
async fn no_child_scope_holds_an_agent_tool_even_when_the_registry_does() {
    let runner = ScriptRunner::new(script(|_, belt| async move {
        let _ = belt;
        Ok(turn("done"))
    }));
    let world = World::new();
    let mut names = crate::fake::world::PRIMITIVES.to_vec();
    names.push("agent_formalize");
    let built = subagents(&world.registry_of(&names), &runner);
    world.add_paper("2012.04125", &["thm:main"]);
    let claim = world.add_claim(json!({"kind": "result", "statement": "s",
        "lean": {"path": PATH, "declaration": "d", "file_sha256": "x"}}));
    world.add_decl("d", None);
    call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "x"}, "path": PATH}),
    )
    .await;
    call(&built, "agent_review_alignment", json!({"claim_id": claim})).await;
    call(&built, "agent_refute", json!({"claim_id": claim})).await;
    let scopes = runner.scopes();
    assert_eq!(scopes.len(), 4);
    for (scope, kind) in scopes.iter().zip(ChildKind::ALL) {
        assert!(
            scope.iter().all(|name| !name.starts_with("agent_")),
            "{scope:?}"
        );
        let mut expected: Vec<&str> = kind.spec().primitives.to_vec();
        expected.sort_unstable();
        assert_eq!(*scope, expected, "{}", kind.spec().tool);
    }
}

#[tokio::test]
async fn a_backend_failure_is_backend_unavailable_with_what_was_spent() {
    let runner = ScriptRunner::new(script(|_, belt| async move {
        belt.call("lit_get_paper", json!({"arxiv_id": "2012.04125"}))
            .await;
        Err("connection refused".to_owned())
    }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &[]);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert_eq!(answer.code(), Some(ErrorCode::BackendUnavailable));
    assert!(answer.error.unwrap().message.contains("connection refused"));
    assert_eq!(answer.usage.unwrap().tool_calls, 1);
}

#[tokio::test(start_paused = true)]
async fn a_child_that_outlives_its_wall_time_is_unknown_never_failed() {
    let runner = ScriptRunner::new(script(|_, belt| async move {
        belt.call("lit_get_paper", json!({"arxiv_id": "2012.04125"}))
            .await;
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        Ok(turn("too late"))
    }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &[]);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125", "budget": {"wall_secs": 1}}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(unknowns(&answer).contains("wall time"), "{answer:?}");
    assert_eq!(answer.usage.unwrap().tool_calls, 1);
}

#[tokio::test]
async fn a_cancelled_call_stops_the_child_and_is_unknown() {
    let runner = ScriptRunner::new(script(|_, _| async {
        std::future::pending::<()>().await;
        Ok(turn("never"))
    }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &[]);
    let call_ctx = ctx();
    call_ctx.cancel.cancel();
    let answer = tool(&built, "agent_digest_paper")
        .call(&call_ctx, json!({"arxiv_id": "2012.04125"}))
        .await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(unknowns(&answer).contains("cancelled"), "{answer:?}");
}

// ------------------------------------------------------- agent_digest_paper

fn arxiv_source() -> Value {
    json!({"arxiv": {"id": "2012.04125", "version": "v2", "label": "thm:main"}})
}

#[test]
fn arxiv_ids_are_recognised_in_both_styles() {
    use super::digest::valid_arxiv_id;
    for good in [
        "2012.04125",
        "2012.04125v3",
        "1501.0001",
        "math/0211159",
        "hep-th/9901001v2",
        "math.GT/0309136",
        "solv-int/9511001",
    ] {
        assert!(valid_arxiv_id(good), "{good}");
    }
    for bad in [
        "",
        "abc",
        "2012.041",
        "12.04125",
        "2012.04125v",
        "math/021159",
        "Math/0211159",
        "math.xx/0211159",
        "2012.04125.1",
        "/0211159",
        "math.GT.X/0211159",
        "2012.0412a",
    ] {
        assert!(!valid_arxiv_id(bad), "{bad}");
    }
}

#[tokio::test]
async fn the_digest_returns_the_ids_the_ledger_holds_grouped_by_kind() {
    let runner = ScriptRunner::new(script(|prompt, belt| async move {
        assert!(
            prompt.contains("2012.04125") && prompt.contains("critical radius"),
            "{prompt}"
        );
        for (kind, statement) in [
            ("fact", "Def 1"),
            ("fact", "Thm 2"),
            ("lead", "Open question"),
        ] {
            let recorded = belt
                .call(
                    "ledger_record",
                    json!({"kind": kind, "statement": statement, "source": arxiv_source()}),
                )
                .await;
            assert_eq!(recorded.outcome, Outcome::Ok);
        }
        Ok(turn(&"word ".repeat(200)))
    }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &[]);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125", "focus": "the bound on the critical radius"}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(answer.data["count"], 3);
    assert_eq!(answer.data["recorded"]["fact"].as_array().unwrap().len(), 2);
    assert_eq!(answer.data["recorded"]["lead"].as_array().unwrap().len(), 1);
    let summary = answer.data["untrusted"]["summary"].as_str().unwrap();
    assert_eq!(summary.split_whitespace().count(), 120);
    assert_eq!(answer.data["session_id"], "sess-1");
    let usage = answer.usage.unwrap();
    assert_eq!(
        (usage.model.as_str(), usage.tool_calls, usage.turns),
        ("scripted", 3, 3)
    );
    assert_eq!((usage.input_tokens, usage.output_tokens), (100, 20));
    assert!(answer.next[0].starts_with("ledger_claims"));
}

#[tokio::test]
async fn a_child_that_claims_records_but_made_none_yields_unknown_with_nothing_recorded() {
    let runner = ScriptRunner::new(script(|_, _| async {
        Ok(turn(
            "I recorded five facts: clm_91, clm_92, clm_93, clm_94, clm_95.",
        ))
    }));
    let (world, built) = setup(&runner);
    world.add_paper("2012.04125", &[]);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert_eq!(answer.data["count"], 0);
    assert!(unknowns(&answer).contains("recorded nothing"));
    assert_eq!(world.records().len(), 0);
}

#[tokio::test]
async fn an_id_the_ledger_lacks_is_dropped_and_counted_in_unknowns() {
    let world = World::new();
    let script_world = world.clone();
    let runner = ScriptRunner::new(script(move |_, belt| {
        let world = script_world.clone();
        async move {
            for statement in ["a", "b"] {
                belt.call(
                    "ledger_record",
                    json!({"kind": "fact", "statement": statement, "source": arxiv_source()}),
                )
                .await;
            }
            world.forget_claim("clm_1");
            Ok(turn("ok"))
        }
    }));
    let built = subagents(&world.registry(), &runner);
    world.add_paper("2012.04125", &[]);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok);
    assert_eq!(answer.data["count"], 1);
    assert!(unknowns(&answer).contains("1 id(s)"), "{answer:?}");
}

/// A reader that records `n` facts and then sees the ledger stop answering.
fn reader_then_outage(world: &World, n: usize) -> Arc<ScriptRunner> {
    let world = world.clone();
    ScriptRunner::new(script(move |_, belt| {
        let world = world.clone();
        async move {
            for statement in ["a", "b", "c"].into_iter().take(n) {
                belt.call(
                    "ledger_record",
                    json!({"kind": "fact", "statement": statement, "source": arxiv_source()}),
                )
                .await;
            }
            world.override_tool(
                "ledger_claims",
                Envelope::error(ErrorCode::BackendUnavailable, "cortex is down"),
            );
            Ok(turn("ok"))
        }
    }))
}

#[tokio::test]
async fn a_ledger_that_cannot_be_read_back_is_unknown_never_missing() {
    let world = World::new();
    world.add_paper("2012.04125", &[]);
    let runner = reader_then_outage(&world, 2);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Unknown, "{answer:?}");
    assert_eq!(answer.data["count"], 0);
    assert_eq!(answer.data["unverified"], json!(["clm_1", "clm_2"]));
    let said = unknowns(&answer);
    assert!(!said.contains("not in the ledger"), "{said}");
    assert!(
        said.contains("2 id(s) could not be read back from the ledger")
            && said.contains("cortex is down"),
        "{said}"
    );
    assert!(answer.next.is_empty());
}

#[tokio::test]
async fn a_status_above_cited_is_flagged_not_trusted() {
    let world = World::new();
    world.force_status("checked");
    let runner = ScriptRunner::new(script(|_, belt| async move {
        belt.call(
            "ledger_record",
            json!({"kind": "fact", "statement": "a", "source": arxiv_source()}),
        )
        .await;
        Ok(turn("ok"))
    }));
    let built = subagents(&world.registry(), &runner);
    world.add_paper("2012.04125", &[]);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert!(
        unknowns(&answer).contains("earns at most cited"),
        "{answer:?}"
    );
}

#[tokio::test]
async fn a_digest_that_runs_out_of_budget_is_unknown_with_what_was_recorded() {
    let world = World::new();
    let script_world = world.clone();
    let runner = ScriptRunner::new(script(move |_, belt| {
        let _ = &script_world;
        async move {
            belt.call(
                "ledger_record",
                json!({"kind": "fact", "statement": "a", "source": arxiv_source()}),
            )
            .await;
            Ok(crate::child::ChildTurn {
                end: ChildEnd::OutOfBudget("iterations".into()),
                unapplied: Vec::new(),
                ..turn("")
            })
        }
    }));
    let built = subagents(&world.registry(), &runner);
    world.add_paper("2012.04125", &[]);
    let answer = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert_eq!(answer.data["count"], 1);
    assert!(unknowns(&answer).contains("ran out of budget (iterations)"));
}

#[tokio::test]
async fn a_malformed_or_unknown_paper_is_refused_before_the_child_runs() {
    let runner = ScriptRunner::new(script(|_, _| async { Ok(turn("never")) }));
    let (world, built) = setup(&runner);
    let bad = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "not-an-id"}),
    )
    .await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));
    let missing = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert_eq!(missing.code(), Some(ErrorCode::NotFound));
    world.override_tool(
        "lit_get_paper",
        Envelope::error(ErrorCode::RateLimited, "slow down"),
    );
    let limited = call(
        &built,
        "agent_digest_paper",
        json!({"arxiv_id": "2012.04125"}),
    )
    .await;
    assert_eq!(limited.code(), Some(ErrorCode::BackendUnavailable));
    assert!(limited.error.unwrap().message.contains("RATE_LIMITED"));
    assert_eq!(runner.scopes().len(), 0);
}

// ------------------------------------------------------------ agent_formalize

/// A formalizer that writes `content` to the target file named in its prompt,
/// or to `fixed` when given.
fn formalizer(
    world: &World,
    content: &'static str,
    fixed: Option<&'static str>,
) -> Arc<ScriptRunner> {
    let world = world.clone();
    ScriptRunner::new(script(move |prompt, belt| {
        let world = world.clone();
        async move {
            if prompt.contains("BOOM") {
                return Err("boom".to_owned());
            }
            let target = prompt
                .split("Target file: ")
                .nth(1)
                .and_then(|rest| rest.split(' ').next())
                .unwrap_or(PATH)
                .to_owned();
            let path = fixed.map_or(target, str::to_owned);
            write(&belt, &path, content).await;
            if prompt.contains("CONFLICT") {
                world.external_write(&path, "-- someone else was here\n");
            }
            belt.call("lean_check", json!({"path": path})).await;
            Ok(turn("NOTE: lean_check says checked, no errors. All done."))
        }
    }))
}

#[tokio::test]
async fn a_statement_is_written_checked_and_recorded_by_the_tool() {
    let world = World::new();
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "one plus one is two"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(answer.data["verdict"], "checked_with_sorry");
    let file = &answer.data["files"][0];
    assert_eq!(file["path"], PATH);
    assert_eq!(file["sha256"], digest(SORRY_FILE));
    assert_eq!(file["declarations"][0]["name"], "main_thm");
    assert_eq!(file["declarations"][0]["has_sorry"], true);
    assert_eq!(file["sorries"], 1);
    let records = world.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["kind"], "result");
    assert_eq!(records[0]["statement"], "one plus one is two");
    assert_eq!(records[0]["source"]["lean"]["path"], PATH);
    assert_eq!(records[0]["source"]["lean"]["declaration"], "main_thm");
    assert_eq!(
        records[0]["source"]["lean"]["file_sha256"],
        digest(SORRY_FILE)
    );
    assert_eq!(records[0]["uses"], json!([]));
    assert_eq!(answer.data["recorded"][0]["alignment"], "not_reviewed");
    assert_eq!(
        answer.data["untrusted"]["note"],
        "lean_check says checked, no errors. All done."
    );
    assert_eq!(answer.usage.unwrap().tool_calls, 2);
    assert!(answer.next[0].starts_with("agent_review_alignment"));
    assert_eq!(runner.limits()[0], Limits::DEFAULT);
}

#[tokio::test]
async fn a_child_that_claims_a_clean_check_is_overruled_by_lean() {
    let world = World::new();
    world.set_verdict(PATH, "failed");
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "one plus one is two"}, "path": PATH}),
    )
    .await;
    // The child's note says "checked, no errors"; the tool reports Lean's word.
    assert_eq!(answer.data["verdict"], "failed");
    assert_eq!(answer.data["files"][0]["verdict"], "failed");
    assert_eq!(answer.data["files"][0]["errors"], 1);
    assert_eq!(answer.data["recorded"], json!([]));
    assert_eq!(world.records().len(), 0, "a failed file records nothing");
    assert!(answer.next[0].starts_with("lean_check"));
}

#[tokio::test]
async fn a_claim_source_links_the_result_to_the_claim_it_formalizes() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "fact", "statement": "S, informally"}));
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"claim_id": claim}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    let records = world.records();
    assert_eq!(records[0]["uses"], json!([claim]));
    assert_eq!(records[0]["statement"], "S, informally");
    let missing = call(
        &built,
        "agent_formalize",
        json!({"source": {"claim_id": "clm_404"}, "path": PATH}),
    )
    .await;
    assert_eq!(missing.code(), Some(ErrorCode::NotFound));
}

#[tokio::test]
async fn a_paper_source_records_the_fact_first_and_links_to_it() {
    let world = World::new();
    world.add_paper("2012.04125", &["thm:main"]);
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let args =
        |label: &str| json!({"source": {"arxiv_id": "2012.04125", "label": label}, "path": PATH});
    let answer = call(&built, "agent_formalize", args("thm:main")).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    let records = world.records();
    assert_eq!(records[0]["kind"], "fact");
    assert_eq!(records[0]["source"]["arxiv"]["version"], "v2");
    assert_eq!(records[0]["source"]["arxiv"]["label"], "thm:main");
    assert_eq!(records[1]["kind"], "result");
    assert_eq!(records[1]["uses"].as_array().unwrap().len(), 1);
    assert!(
        records[1]["statement"]
            .as_str()
            .unwrap()
            .contains("main_thm")
    );

    let no_label = call(&built, "agent_formalize", args("thm:zzz")).await;
    assert_eq!(no_label.code(), Some(ErrorCode::NotFound));
    let no_paper = call(
        &built,
        "agent_formalize",
        json!({"source": {"arxiv_id": "1111.11111", "label": "x"}, "path": PATH}),
    )
    .await;
    assert_eq!(no_paper.code(), Some(ErrorCode::NotFound));
}

#[tokio::test]
async fn a_label_on_a_later_page_of_the_statement_list_is_found() {
    let world = World::new();
    // lit_extract_statements pages 50 rows at a time; the label is on page 2.
    let labels: Vec<String> = (0..120).map(|n| format!("thm:{n}")).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    world.add_paper("2012.04125", &labels);
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let args =
        |label: &str| json!({"source": {"arxiv_id": "2012.04125", "label": label}, "path": PATH});
    let late = call(&built, "agent_formalize", args("thm:119")).await;
    assert_eq!(late.outcome, Outcome::Ok, "{late:?}");
    let missing = call(&built, "agent_formalize", args("thm:120")).await;
    assert_eq!(missing.code(), Some(ErrorCode::NotFound), "{missing:?}");
}

#[tokio::test]
async fn declarations_are_recorded_under_the_names_lean_knows_them_by() {
    // file_read's outline names a declaration as written; inside a namespace
    // the ledger and lean_check need it qualified.
    const NAMESPACED: &str = "import Mathlib\n\nnamespace Sendov\n\ntheorem main_thm : 1 + 1 = 2 := by\n  sorry\n\nend Sendov\n";
    let world = World::new();
    let runner = formalizer(&world, NAMESPACED, None);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "one plus one is two"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(
        answer.data["files"][0]["declarations"][0]["name"],
        "Sendov.main_thm"
    );
    let records = world.records();
    assert_eq!(
        records[0]["source"]["lean"]["declaration"],
        "Sendov.main_thm"
    );
    assert_eq!(answer.data["recorded"][0]["declaration"], "Sendov.main_thm");
}

#[test]
fn outline_names_are_matched_to_targets_in_file_order() {
    use super::verify::{Decl, LeanVerdict, Outline};
    let decl = |name: &str, kind: &str| Decl {
        name: name.into(),
        kind: kind.into(),
        has_sorry: Some(false),
    };
    let outline = Outline {
        sha256: "s".into(),
        decls: vec![
            decl("foo", "theorem"),
            decl("<anonymous>", "example"),
            decl("foo", "lemma"),
            decl("_root_.top", "theorem"),
            decl("gone", "theorem"),
        ],
    };
    let verdict = LeanVerdict {
        verdict: "checked".into(),
        file_sha256: None,
        targets: vec!["A.foo".into(), "B.foo".into(), "top".into()],
        errors: 0,
        sorries: 0,
        debt: Vec::new(),
        reason: None,
    };
    let names: Vec<Option<String>> = outline
        .qualified(&verdict)
        .into_iter()
        .map(|(_, name)| name)
        .collect();
    assert_eq!(
        names,
        [
            Some("A.foo".into()),
            None,
            Some("B.foo".into()),
            Some("top".into()),
            None
        ]
    );
}

#[tokio::test]
async fn lean_errors_are_counted_past_the_twenty_the_verdict_lists() {
    let world = World::new();
    world.override_tool(
        "lean_check",
        Envelope::ok(json!({
            "verdict": "failed",
            "provenance": {"path": PATH, "file_sha256": "abc"},
            "targets": [],
            "errors": [{"line": 1, "column": 0, "message": "e"}],
            "error_count": 57,
            "sorries": [],
        })),
    );
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "s"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.data["files"][0]["errors"], 57);
}

#[tokio::test]
async fn the_label_is_not_checked_when_the_registry_cannot_list_statements() {
    let world = World::new();
    world.add_paper("2012.04125", &["thm:main"]);
    let runner = formalizer(&world, SORRY_FILE, None);
    let names: Vec<&'static str> = crate::fake::world::PRIMITIVES
        .iter()
        .copied()
        .filter(|name| *name != "lit_extract_statements")
        .collect();
    let built = subagents(&world.registry_of(&names), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"arxiv_id": "2012.04125", "label": "anything"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
}

#[tokio::test]
async fn a_path_changed_under_another_writer_is_a_conflict() {
    let world = World::new();
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "CONFLICT"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.code(), Some(ErrorCode::Conflict));
    assert_eq!(answer.usage.unwrap().tool_calls, 2);
    assert_eq!(world.records().len(), 0);
}

#[tokio::test]
async fn a_child_that_wrote_nothing_or_the_wrong_file_is_unknown() {
    let world = World::new();
    let idle = ScriptRunner::new(script(|_, _| async {
        Ok(turn("NOTE: the Mathlib definition is missing"))
    }));
    let built = subagents(&world.registry(), &idle);
    let nothing = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "x"}, "path": PATH}),
    )
    .await;
    assert_eq!(nothing.outcome, Outcome::Unknown);
    assert!(unknowns(&nothing).contains("wrote no file"));
    assert_eq!(
        nothing.data["untrusted"]["note"],
        "the Mathlib definition is missing"
    );

    let wrong = formalizer(&world, SORRY_FILE, Some("lean/Supermath/Helper.lean"));
    let built = subagents(&world.registry(), &wrong);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "x"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(unknowns(&answer).contains("did not write the target"));
    assert_eq!(
        answer.data["files"][0]["path"],
        "lean/Supermath/Helper.lean"
    );
    assert_eq!(answer.data["verdict"], Value::Null);
    assert_eq!(world.records().len(), 0, "a helper file is not a target");
}

#[tokio::test]
async fn proof_depth_gets_more_iterations_and_flags_remaining_sorries() {
    let world = World::new();
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let args = json!({"source": {"statement": "x"}, "depth": "proof", "path": PATH});
    let answer = call(&built, "agent_formalize", args).await;
    assert_eq!(runner.limits()[0].max_iterations, 48);
    assert_eq!(answer.data["depth"], "proof");
    assert!(
        unknowns(&answer).contains("1 sorry(s) remain"),
        "{answer:?}"
    );
    let explicit = json!({"source": {"statement": "x"}, "depth": "proof", "path": "lean/B.lean",
        "budget": {"max_iterations": 10}});
    call(&built, "agent_formalize", explicit).await;
    assert_eq!(runner.limits()[1].max_iterations, 10);
}

#[tokio::test]
async fn an_edited_file_is_read_back_at_its_newest_digest() {
    let world = World::new();
    let runner = ScriptRunner::new(script(|_, belt| async move {
        write(&belt, PATH, SORRY_FILE).await;
        let edit = belt
            .call(
                "file_edit",
                json!({
                    "path": PATH,
                    "edits": [{"old": "sorry", "new": "norm_num"}],
                    "if_sha256": digest(SORRY_FILE),
                }),
            )
            .await;
        assert_eq!(edit.outcome, Outcome::Ok);
        Ok(turn("NOTE: none"))
    }));
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "x"}, "depth": "proof", "path": PATH}),
    )
    .await;
    assert_eq!(answer.data["files"].as_array().unwrap().len(), 1);
    assert_eq!(answer.data["verdict"], "checked");
    assert_eq!(answer.unknowns.len(), 0, "{answer:?}");
}

#[tokio::test]
async fn an_unreadable_file_and_an_unknown_verdict_are_reported_not_guessed() {
    let world = World::new();
    world.set_verdict(PATH, "unknown");
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let args = json!({"source": {"statement": "x"}, "path": PATH});
    let timed_out = call(&built, "agent_formalize", args.clone()).await;
    assert_eq!(timed_out.outcome, Outcome::Unknown);
    assert_eq!(timed_out.data["verdict"], "unknown");
    assert!(unknowns(&timed_out).contains("lean_check timed out"));
    assert_eq!(world.records().len(), 0);

    let unreadable = ScriptRunner::new(script({
        let world = world.clone();
        move |_, belt| {
            let world = world.clone();
            async move {
                write(&belt, "lean/C.lean", SORRY_FILE).await;
                world.override_tool(
                    "file_read",
                    Envelope::error(ErrorCode::BackendUnavailable, "disk"),
                );
                Ok(turn("NOTE: none"))
            }
        }
    }));
    let built = subagents(&world.registry(), &unreadable);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "x"}, "path": "lean/C.lean"}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(unknowns(&answer).contains("could not be read back"));
}

#[tokio::test]
async fn a_ledger_that_refuses_leaves_the_files_reported_and_the_failure_named() {
    let world = World::new();
    world.add_paper("2012.04125", &["thm:main"]);
    world.refuse_records();
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"arxiv_id": "2012.04125", "label": "thm:main"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok);
    assert_eq!(answer.data["recorded"], json!([]));
    assert!(unknowns(&answer).contains("source fact was not recorded"));
    assert!(unknowns(&answer).contains("main_thm was not recorded"));
}

#[tokio::test]
async fn a_formalizer_out_of_budget_is_unknown_with_its_files_and_verdicts() {
    let world = World::new();
    let runner = ScriptRunner::new(script(|_, belt| async move {
        write(&belt, PATH, SORRY_FILE).await;
        Ok(crate::child::ChildTurn {
            end: ChildEnd::OutOfBudget("tokens".into()),
            unapplied: Vec::new(),
            ..turn("")
        })
    }));
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_formalize",
        json!({"source": {"statement": "x"}, "path": PATH}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(unknowns(&answer).contains("ran out of budget (tokens)"));
    assert_eq!(answer.data["files"][0]["verdict"], "checked_with_sorry");
}

#[tokio::test]
async fn sources_and_paths_that_do_not_make_sense_are_invalid_input() {
    use super::formalize::valid_lean_path;
    let runner = ScriptRunner::new(script(|_, _| async { Ok(turn("never")) }));
    let (_, built) = setup(&runner);
    for source in [
        json!({}),
        json!({"statement": "x", "claim_id": "clm_1"}),
        json!({"arxiv_id": "2012.04125"}),
        json!({"arxiv_id": "bogus", "label": "x"}),
        json!({"statement": "   "}),
        json!({"claim_id": ""}),
        json!({"surprise": 1}),
    ] {
        let answer = call(
            &built,
            "agent_formalize",
            json!({"source": source, "path": PATH}),
        )
        .await;
        assert_eq!(answer.code(), Some(ErrorCode::InvalidInput), "{source}");
    }
    for path in [
        "Main.lean",
        "lean/Main.py",
        "lean/../x.lean",
        "/lean/x.lean",
        "lean/a b.lean",
        "lean//x.lean",
        "lean/.hidden.lean",
        "python/x.lean",
    ] {
        assert!(!valid_lean_path(path), "{path}");
        let answer = call(
            &built,
            "agent_formalize",
            json!({"source": {"statement": "x"}, "path": path}),
        )
        .await;
        assert_eq!(answer.code(), Some(ErrorCode::InvalidInput), "{path}");
    }
    for path in [PATH, "lean/A-b_c.1.lean"] {
        assert!(valid_lean_path(path), "{path}");
    }
    assert_eq!(runner.scopes().len(), 0);
}

// ------------------------------------------------------ agent_review_alignment

/// A world with a fact, a Lean result using it, and the declaration's file.
fn reviewed_world() -> (World, String) {
    let world = World::new();
    let fact = world.add_claim(json!({"kind": "fact", "statement": "1 + 1 = 2, informally"}));
    world.add_decl("Main.main_thm", Some(PATH));
    world.external_write(PATH, SORRY_FILE);
    let claim = world.add_claim(json!({
        "kind": "result", "statement": "formal statement",
        "lean": {"path": PATH, "declaration": "Main.main_thm", "file_sha256": "x"},
        "uses": [fact],
    }));
    (world, claim)
}

fn reviewer(reply: &'static str) -> Arc<ScriptRunner> {
    ScriptRunner::new(script(move |prompt, _| async move {
        if prompt.contains("BOOM") {
            return Err("boom".to_owned());
        }
        Ok(turn(reply))
    }))
}

#[tokio::test]
async fn an_accepted_review_is_recorded_against_the_claim_with_the_reviewers_session() {
    let (world, claim) = reviewed_world();
    let runner = ScriptRunner::new(script(|prompt, belt| async move {
        assert!(prompt.contains("1 + 1 = 2, informally"), "{prompt}");
        assert!(prompt.contains("Main.main_thm"), "{prompt}");
        belt.call("lean_decl", json!({"name": "Main.main_thm"}))
            .await;
        Ok(turn("VERDICT: accepted"))
    }));
    let built = subagents(&world.registry(), &runner);
    let answer = call(&built, "agent_review_alignment", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(answer.data["alignment"], "accepted");
    assert_eq!(answer.data["lean_verdict"], "checked_with_sorry");
    assert_eq!(answer.data["reviewer"]["model"], "scripted");
    assert_eq!(answer.data["reviewer"]["session_id"], "sess-1");
    let records = world.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["uses"], json!([claim]));
    assert!(
        records[0]["statement"]
            .as_str()
            .unwrap()
            .contains("accepted")
    );
    assert!(
        records[0]["source"]["supplied"]
            .as_str()
            .unwrap()
            .contains("scripted")
    );
    assert_eq!(answer.data["review_id"], "clm_3");
    assert_eq!(runner.limits()[0], Limits::REVIEW);
}

#[tokio::test]
async fn a_disputed_review_returns_the_specific_mismatch_as_untrusted_prose() {
    let (world, claim) = reviewed_world();
    let built = subagents(
        &world.registry(),
        &reviewer(
            "VERDICT: disputed\nMISMATCH: the hypothesis n > 0 is missing\nand subtraction truncates",
        ),
    );
    let answer = call(&built, "agent_review_alignment", json!({"claim_id": claim})).await;
    assert_eq!(answer.data["alignment"], "disputed");
    let mismatch = answer.data["untrusted"]["mismatch"].as_str().unwrap();
    assert_eq!(
        mismatch,
        "the hypothesis n > 0 is missing and subtraction truncates"
    );
    assert!(
        world.records()[0]["statement"]
            .as_str()
            .unwrap()
            .contains("n > 0")
    );
}

#[tokio::test]
async fn a_reviewer_that_accepts_a_file_lean_rejects_leaves_alignment_unreviewed() {
    let (world, claim) = reviewed_world();
    world.set_verdict(PATH, "failed");
    let built = subagents(&world.registry(), &reviewer("VERDICT: accepted"));
    let answer = call(&built, "agent_review_alignment", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert_eq!(answer.data["alignment"], "not_reviewed");
    assert_eq!(answer.data["lean_verdict"], "failed");
    assert!(unknowns(&answer).contains("failed in Lean"), "{answer:?}");
    assert_eq!(world.records().len(), 0);
}

#[tokio::test]
async fn a_reply_that_is_neither_accepted_nor_disputed_is_unknown() {
    let (world, claim) = reviewed_world();
    let built = subagents(&world.registry(), &reviewer("It looks fine to me."));
    let answer = call(&built, "agent_review_alignment", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert_eq!(answer.data["alignment"], "not_reviewed");
    assert!(unknowns(&answer).contains("neither accepted nor disputed"));
    assert_eq!(world.records().len(), 0);
}

#[tokio::test]
async fn a_declaration_and_statement_are_reviewed_without_a_claim_to_update() {
    let (world, _) = reviewed_world();
    let built = subagents(&world.registry(), &reviewer("VERDICT: accepted"));
    let args = json!({"declaration": "Main.main_thm", "statement": "1 + 1 = 2"});
    let answer = call(&built, "agent_review_alignment", args).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(answer.data["alignment"], "accepted");
    assert_eq!(answer.data["lean_verdict"], "checked_with_sorry");
    assert_eq!(answer.data["review_id"], Value::Null);
    assert_eq!(world.records().len(), 0);

    // A declaration with no project file has no Lean verdict to re-run.
    world.add_decl("Mathlib.thing", None);
    let mathlib = json!({"declaration": "Mathlib.thing", "statement": "s"});
    let answer = call(&built, "agent_review_alignment", mathlib).await;
    assert_eq!(answer.outcome, Outcome::Ok);
    assert_eq!(answer.data["lean_verdict"], Value::Null);

    let missing = json!({"declaration": "No.such", "statement": "s"});
    let answer = call(&built, "agent_review_alignment", missing).await;
    assert_eq!(answer.code(), Some(ErrorCode::NotFound));
}

#[tokio::test]
async fn claims_the_tool_cannot_review_are_refused_with_the_reason() {
    let (world, _) = reviewed_world();
    let built = subagents(&world.registry(), &reviewer("VERDICT: accepted"));
    let no_lean = world.add_claim(json!({"kind": "fact", "statement": "just a fact"}));
    let no_decl =
        world.add_claim(json!({"kind": "result", "statement": "s", "lean": {"path": PATH}}));
    let from_source = world.add_claim(json!({
        "kind": "result", "statement": "stated in source only", "lean": null,
        "source": {"lean": {"path": PATH, "declaration": "Main.main_thm", "file_sha256": "x"}},
    }));
    for (args, code) in [
        (json!({"claim_id": no_lean}), ErrorCode::InvalidInput),
        (json!({"claim_id": no_decl}), ErrorCode::InvalidInput),
        (json!({"claim_id": "clm_404"}), ErrorCode::NotFound),
        (json!({}), ErrorCode::InvalidInput),
        (
            json!({"declaration": "Main.main_thm"}),
            ErrorCode::InvalidInput,
        ),
        (
            json!({"declaration": "Main.main_thm", "statement": " "}),
            ErrorCode::InvalidInput,
        ),
        (
            json!({"claim_id": "clm_1", "declaration": "d", "statement": "s"}),
            ErrorCode::InvalidInput,
        ),
    ] {
        let answer = call(&built, "agent_review_alignment", args.clone()).await;
        assert_eq!(answer.code(), Some(code), "{args}");
    }
    // A claim whose Lean source sits under `source`, and no `uses`, falls back
    // to the claim's own statement.
    let runner = ScriptRunner::new(script(|prompt, _| async move {
        assert!(prompt.contains("stated in source only"), "{prompt}");
        Ok(turn("VERDICT: accepted"))
    }));
    let built = subagents(&world.registry(), &runner);
    let answer = call(
        &built,
        "agent_review_alignment",
        json!({"claim_id": from_source}),
    )
    .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
}

#[tokio::test]
async fn a_ledger_that_refuses_the_review_still_returns_the_judgment() {
    let (world, claim) = reviewed_world();
    world.refuse_records();
    let built = subagents(&world.registry(), &reviewer("VERDICT: accepted"));
    let answer = call(&built, "agent_review_alignment", json!({"claim_id": claim})).await;
    assert_eq!(answer.data["alignment"], "accepted");
    assert_eq!(answer.data["review_id"], Value::Null);
    assert!(unknowns(&answer).contains("review was not recorded"));
}

#[tokio::test]
async fn a_reviewer_out_of_budget_is_unknown_and_the_reviewer_cannot_read_files() {
    let (world, claim) = reviewed_world();
    let runner = ScriptRunner::new(script(|_, belt| async move {
        for forbidden in ["file_read", "lean_goals", "lean_check", "file_list"] {
            let answer = belt.call(forbidden, json!({"path": PATH})).await;
            assert_eq!(answer.code(), Some(ErrorCode::NotFound), "{forbidden}");
        }
        Ok(crate::child::ChildTurn {
            end: ChildEnd::OutOfBudget("iterations".into()),
            unapplied: Vec::new(),
            ..turn("VERDICT: accepted")
        })
    }));
    let built = subagents(&world.registry(), &runner);
    let answer = call(&built, "agent_review_alignment", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(unknowns(&answer).contains("ran out of budget"));
    assert_eq!(
        runner.scopes()[0],
        ["lean_decl", "lean_search", "lit_get_paper", "recall_memory"]
    );
}

#[test]
fn judgments_are_read_from_the_verdict_line() {
    use super::review::{Judgment, parse_judgment};
    assert_eq!(parse_judgment("VERDICT: accepted"), Judgment::Accepted);
    assert_eq!(
        parse_judgment("notes\n**VERDICT:** accepted\n"),
        Judgment::Accepted
    );
    assert_eq!(parse_judgment("verdict: Accepted."), Judgment::Accepted);
    assert_eq!(
        parse_judgment("VERDICT: disputed\nMISMATCH: a\n\nb"),
        Judgment::Disputed("a b".into())
    );
    assert_eq!(
        parse_judgment("VERDICT: disputed"),
        Judgment::Disputed(String::new())
    );
    assert_eq!(
        parse_judgment("VERDICT: disputed\nVERDICT: accepted"),
        Judgment::Accepted
    );
    assert_eq!(parse_judgment("VERDICT: maybe"), Judgment::Unclear);
    assert_eq!(parse_judgment("looks fine"), Judgment::Unclear);
    let long = format!("VERDICT: disputed\nMISMATCH: {}", "w ".repeat(300));
    let Judgment::Disputed(text) = parse_judgment(&long) else {
        panic!("disputed")
    };
    assert_eq!(text.split_whitespace().count(), 100);
}

// ------------------------------------------------------------- agent_refute

fn refuter(reply: &'static str) -> Arc<ScriptRunner> {
    ScriptRunner::new(script(move |prompt, belt| async move {
        if prompt.contains("BOOM") {
            return Err("boom".to_owned());
        }
        belt.call("file_write", json!({"path": "lean/Supermath/Search.lean", "content": "def P (n : Nat) : Prop := n < 7\n"})).await;
        belt.call(
            "py_search_counterexample",
            json!({
                "path": "python/work/search.py",
                "predicate": "p",
                "frame": {"kind": "range", "from": 0, "to": 100},
            }),
        )
        .await;
        belt.call(
            "lean_search_counterexample",
            json!({
                "path": "lean/Supermath/Search.lean",
                "predicate": "P",
                "range": {"from": 0, "to": 100},
            }),
        )
        .await;
        Ok(turn(reply))
    }))
}

/// `py_search_counterexample`'s answer for a witness: `data.result` is
/// `found`, and the generated file is `{path, sha256}`.
fn python_found(witness: i64) -> Envelope {
    Envelope::ok(json!({
        "result": "found",
        "frame": {"kind": "range", "from": 0, "to": 100},
        "frame_description": "n in 0..=100",
        "witness": witness,
        "witness_file": {"path": "python/work/witnesses/p_1.py", "sha256": "ab"},
    }))
}

/// `lean_search_counterexample`'s answer for a witness: `data.witness_file`
/// is a path, `data.theorem` the generated theorem, and the frame an object.
fn lean_found(witness: i64, file: &str, theorem: &str) -> Envelope {
    Envelope::ok(json!({
        "found": true,
        "witness": witness,
        "frame": {"predicate": "P", "from": 0, "to": 100, "covered_to": witness, "domain": "nat"},
        "witness_file": file,
        "theorem": theorem,
        "evidence_origin": "collected",
        "verdict": "checked",
        "provenance": {"path": file, "file_sha256": "unused"},
        "errors": [],
    }))
}

/// `lean_search_counterexample`'s answer when the range holds no witness.
fn lean_exhausted(covered_to: i64) -> Envelope {
    Envelope::ok(json!({
            "found": false,
            "result": "exhausted",
            "frame": {"predicate": "P", "from": 0, "to": 100, "covered_to": covered_to, "domain": "nat"},
        }))
        .with_coverage(Coverage::new(["decide over ℕ in 0..=100"], ["values outside the range"]))
}

fn found(world: &World) {
    world.external_write(WITNESS, WITNESS_FILE);
    world.override_tool(
        "lean_search_counterexample",
        lean_found(7, WITNESS, "notp_witness"),
    );
    world.override_tool("py_search_counterexample", python_found(7));
}

#[tokio::test]
async fn a_counterexample_counts_only_after_the_tool_confirms_it_in_lean() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "P holds for every n"}));
    found(&world);
    let built = subagents(
        &world.registry(),
        &refuter("Found it.\nREPAIRED: assume n is at most 6 in the statement"),
    );
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    let example = &answer.data["counterexample"];
    assert_eq!(example["witness"], "7");
    assert_eq!(example["path"], WITNESS);
    assert_eq!(example["verdict"], "checked");
    assert_eq!(example["file_sha256"], digest(WITNESS_FILE));
    assert_eq!(example["declaration"], "notp_witness");
    let records = world.records();
    assert_eq!(records[0]["kind"], "refutation");
    assert_eq!(records[0]["refutes"], json!(claim));
    assert_eq!(records[0]["source"]["lean"]["declaration"], "notp_witness");
    assert_eq!(records[1]["kind"], "lead");
    assert!(
        records[1]["statement"]
            .as_str()
            .unwrap()
            .contains("at most 6")
    );
    assert_eq!(
        answer.data["untrusted"]["repaired_statement"]["statement"],
        "assume n is at most 6 in the statement"
    );
    assert_eq!(answer.data["leads"], json!([]));
    assert_eq!(records.len(), 2);
}

#[tokio::test]
async fn a_child_that_claims_a_refutation_lean_rejects_leaves_only_a_lead() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "P holds for every n"}));
    found(&world);
    world.set_verdict(WITNESS, "failed");
    let built = subagents(
        &world.registry(),
        &refuter("REFUTED: n = 7 is a counterexample."),
    );
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert_eq!(answer.data["counterexample"], Value::Null);
    assert!(
        unknowns(&answer).contains("failed on re-check"),
        "{answer:?}"
    );
    let records = world.records();
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0]["kind"], "lead");
    assert!(
        records[0]["statement"]
            .as_str()
            .unwrap()
            .contains("not confirmed in Lean")
    );
    assert_eq!(answer.data["leads"][0]["witness"], "7");
}

#[tokio::test]
async fn nothing_found_records_every_frame_searched_as_a_bound() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "P holds for every n"}));
    world.override_tool(
        "py_search_counterexample",
        Envelope::ok(json!({
            "result": "exhausted",
            "frame": {"kind": "range", "from": 0, "to": 1000},
            "frame_description": "n in 0..=1000",
            "checked": 1001,
        })),
    );
    world.override_tool("lean_search_counterexample", lean_exhausted(100));
    let built = subagents(&world.registry(), &refuter("REPAIRED: none"));
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    let frames = answer.data["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 2);
    let records = world.records();
    assert!(records.iter().all(|r| r["kind"] == "bound"));
    // A search that covered its whole frame ended `ok`, so the bound is
    // sourced from the call itself and the ledger earns `checked` on it.
    let source = &records[0]["source"]["tool"];
    assert_eq!(source["tool"], "py_search_counterexample");
    assert!(
        source["call_id"]
            .as_str()
            .unwrap()
            .contains(".py_search_counterexample."),
        "{source}"
    );
    assert_eq!(
        records[1]["source"]["tool"]["tool"],
        "lean_search_counterexample"
    );
    assert_eq!(frames[0]["bound"]["status"], "checked");
    assert_eq!(records[0]["frame"], "n in 0..=1000");
    assert_eq!(records[1]["frame"], "decide P over ℕ in 0..=100");
    assert_eq!(answer.data["untrusted"]["repaired_statement"], Value::Null);
}

#[tokio::test]
async fn the_record_cap_counts_only_witnesses_and_frames_and_reports_what_it_dropped() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "P holds for every n"}));
    let script_world = world.clone();
    let runner = ScriptRunner::new(script(move |_, belt| {
        let world = script_world.clone();
        async move {
            let py = || {
                json!({"path": "python/work/search.py", "predicate": "p",
                       "frame": {"kind": "range", "from": 0, "to": 100}})
            };
            let lean = || {
                json!({"path": "lean/Supermath/Search.lean", "predicate": "P",
                       "range": {"from": 0, "to": 100}})
            };
            // Three searches that found nothing and stated no frame come
            // first; they must not use up the cap.
            for _ in 0..3 {
                belt.call("py_search_counterexample", py()).await;
            }
            world.override_tool("py_search_counterexample", python_found(7));
            for _ in 0..6 {
                belt.call("py_search_counterexample", py()).await;
            }
            world.override_tool("lean_search_counterexample", lean_exhausted(100));
            for _ in 0..7 {
                belt.call("lean_search_counterexample", lean()).await;
            }
            Ok(turn("nothing confirmed"))
        }
    }));
    let built = subagents(&world.registry(), &runner);
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Unknown, "{answer:?}");
    assert_eq!(answer.data["leads"].as_array().unwrap().len(), 5);
    assert_eq!(answer.data["frames"].as_array().unwrap().len(), 5);
    let said = unknowns(&answer);
    assert!(
        said.contains("1 more Python witness(es) were not recorded as leads (at most 5 per call)"),
        "{said}"
    );
    assert!(
        said.contains("2 more frame(s) searched to the end were not recorded as bounds (at most 5 per call)"),
        "{said}"
    );
}

#[tokio::test]
async fn a_search_that_stopped_early_records_no_frame() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "s"}));
    // A Python search that hit a limit, and a Lean search that timed out
    // before covering any value: neither states a frame it finished.
    world.override_tool(
        "py_search_counterexample",
        Envelope::unknown("limit").with_data(json!({
            "result": "limit", "frame_description": "n in 0..=1000", "checked": 10,
        })),
    );
    world.override_tool(
        "lean_search_counterexample",
        Envelope::unknown("timed out").with_data(json!({
            "found": false,
            "frame": {"predicate": "P", "from": 0, "to": 100, "covered_to": null, "domain": ""},
        })),
    );
    let built = subagents(&world.registry(), &refuter("REPAIRED: none"));
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.data["frames"], json!([]));
    assert_eq!(world.records(), Vec::<Value>::new());
}

#[tokio::test]
async fn a_lean_search_cut_short_part_way_records_no_frame() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "s"}));
    world.override_tool(
        "lean_search_counterexample",
        Envelope::unknown("range ran out of time").with_data(json!({
            "found": false,
            "result": "limit",
            "frame": {"predicate": "P", "from": 0, "to": 100, "covered_to": 40, "domain": "nat"},
        })),
    );
    let built = subagents(&world.registry(), &refuter("REPAIRED: none"));
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.data["frames"], json!([]));
    assert_eq!(world.records(), Vec::<Value>::new());
}

#[tokio::test]
async fn the_fake_ledger_refuses_a_tool_source_the_real_one_would() {
    let world = World::new();
    world.override_tool("py_search_counterexample", Envelope::unknown("limit"));
    let registry = world.registry();
    let ctx = CallContext::new(Project::new("p", "ws:t").unwrap(), "root");
    let mut ctx_limit = ctx.clone();
    ctx_limit.call_id = "limit-call".into();
    let args = json!({"path": "python/work/a.py", "predicate": "p",
        "frame": {"kind": "range", "from": 0, "to": 1}});
    let limited = registry.get("py_search_counterexample").unwrap();
    assert_eq!(
        limited.call(&ctx_limit, args).await.outcome,
        Outcome::Unknown
    );
    let ledger = registry.get("ledger_record").unwrap();
    let record = |call_id: &str, tool: &str| {
        json!({"kind": "bound", "statement": "none", "frame": "f",
            "source": {"tool": {"tool": tool, "call_id": call_id}}})
    };
    for (call_id, tool) in [
        ("limit-call", "py_search_counterexample"),
        ("never", "py_run"),
    ] {
        let answer = ledger.call(&ctx, record(call_id, tool)).await;
        assert_eq!(answer.code(), Some(ErrorCode::Forbidden), "{call_id}");
    }
}

#[tokio::test]
async fn a_lean_search_over_the_integers_records_its_frame_in_the_integers() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "s"}));
    world.override_tool(
        "lean_search_counterexample",
        Envelope::ok(json!({
            "found": false,
            "result": "exhausted",
            "frame": {"predicate": "Q", "from": -5, "to": 100, "covered_to": 100, "domain": "int"},
        })),
    );
    let built = subagents(&world.registry(), &refuter("REPAIRED: none"));
    call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(world.records()[0]["frame"], "decide Q over ℤ in -5..=100");
}

#[tokio::test]
async fn witness_files_that_cannot_be_recorded_are_explained_in_unknowns() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "s"}));
    // A witness file that does not exist; a witness theorem the file does not
    // check; a ledger that refuses.
    world.override_tool(
        "lean_search_counterexample",
        lean_found(1, "lean/Supermath/Witnesses/Gone.lean", "gone_witness"),
    );
    let built = subagents(&world.registry(), &refuter("REPAIRED: none"));
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert!(
        unknowns(&answer).contains("unknown on re-check"),
        "{answer:?}"
    );

    // Without the file or theorem the search names, there is nothing to check.
    world.override_tool(
        "lean_search_counterexample",
        Envelope::ok(json!({"found": true, "witness": 1})),
    );
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.data["counterexample"], Value::Null);
    assert_eq!(world.records(), Vec::<Value>::new());

    world.external_write(WITNESS, WITNESS_FILE);
    world.override_tool(
        "lean_search_counterexample",
        lean_found(1, WITNESS, "notp_witness"),
    );
    world.refuse_records();
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert!(
        unknowns(&answer).contains("refutation was not recorded"),
        "{answer:?}"
    );
    assert_eq!(answer.outcome, Outcome::Unknown);
}

#[tokio::test]
async fn a_refuter_out_of_budget_is_unknown_and_unknown_claims_are_not_found() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "s"}));
    let runner = ScriptRunner::new(script(|_, _| async {
        Ok(crate::child::ChildTurn {
            end: ChildEnd::OutOfBudget("wall time".into()),
            unapplied: Vec::new(),
            ..turn("")
        })
    }));
    let built = subagents(&world.registry(), &runner);
    let answer = call(&built, "agent_refute", json!({"claim_id": claim})).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(unknowns(&answer).contains("ran out of budget (wall time)"));
    let missing = call(&built, "agent_refute", json!({"claim_id": "clm_404"})).await;
    assert_eq!(missing.code(), Some(ErrorCode::NotFound));
}

// ------------------------------------------------------------ the contract

fn fixture(args: Value) -> Fixture {
    Fixture::with_ctx(ctx(), args)
}

fn broke() -> CallContext {
    ctx().with_budget(Budget::agentic(1, 1))
}

#[tokio::test]
async fn agent_digest_paper_keeps_the_tool_contract() {
    let world = World::new();
    world.add_paper("2012.04125", &[]);
    let runner = ScriptRunner::new(script(|prompt, belt| async move {
        if prompt.contains("BOOM") {
            return Err("boom".to_owned());
        }
        belt.call(
            "ledger_record",
            json!({"kind": "fact", "statement": "f", "source": arxiv_source()}),
        )
        .await;
        Ok(turn("done"))
    }));
    let built = subagents(&world.registry(), &runner);
    let args = |extra: Value| {
        let mut base = json!({"arxiv_id": "2012.04125"});
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        base
    };
    let fixtures = [
        fixture(args(json!({}))),
        fixture(json!({"arxiv_id": "nope"})),
        fixture(json!({"arxiv_id": "1111.11111"})),
        fixture(args(json!({"focus": "BOOM"}))),
        Fixture::with_ctx(broke(), args(json!({}))),
        Fixture::with_ctx(other_project(), args(json!({}))),
    ];
    contract::check(
        tool(&built, "agent_digest_paper").as_ref(),
        "agent",
        &fixtures,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn agent_formalize_keeps_the_tool_contract() {
    let world = World::new();
    world.add_paper("2012.04125", &["thm:main"]);
    let runner = formalizer(&world, SORRY_FILE, None);
    let built = subagents(&world.registry(), &runner);
    let stmt = |text: &str, path: &str| json!({"source": {"statement": text}, "path": path});
    let fixtures = [
        fixture(stmt("ok", "lean/Supermath/Contract.lean")),
        fixture(json!({"source": {}, "path": "lean/Supermath/Contract.lean"})),
        fixture(json!({"source": {"claim_id": "clm_404"}, "path": "lean/Supermath/Contract.lean"})),
        fixture(stmt("CONFLICT", "lean/Supermath/Racy.lean")),
        fixture(stmt("BOOM", "lean/Supermath/Contract.lean")),
        Fixture::with_ctx(broke(), stmt("ok", "lean/Supermath/Contract.lean")),
        Fixture::with_ctx(other_project(), stmt("ok", "lean/Supermath/Contract.lean")),
    ];
    contract::check(tool(&built, "agent_formalize").as_ref(), "agent", &fixtures)
        .await
        .unwrap();
}

#[tokio::test]
async fn agent_review_alignment_keeps_the_tool_contract() {
    let (world, claim) = reviewed_world();
    let boom = world.add_claim(json!({
        "kind": "result", "statement": "BOOM",
        "lean": {"path": PATH, "declaration": "Main.main_thm", "file_sha256": "x"},
    }));
    let built = subagents(&world.registry(), &reviewer("VERDICT: accepted"));
    let by = |id: &str| json!({"claim_id": id});
    let fixtures = [
        fixture(by(&claim)),
        fixture(by("clm_404")),
        fixture(json!({"claim_id": 5})),
        fixture(by(&boom)),
        Fixture::with_ctx(broke(), by(&claim)),
        Fixture::with_ctx(other_project(), by(&claim)),
    ];
    contract::check(
        tool(&built, "agent_review_alignment").as_ref(),
        "agent",
        &fixtures,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn agent_refute_keeps_the_tool_contract() {
    let world = World::new();
    let claim = world.add_claim(json!({"kind": "result", "statement": "P holds"}));
    let boom = world.add_claim(json!({"kind": "result", "statement": "BOOM"}));
    found(&world);
    let built = subagents(&world.registry(), &refuter("REPAIRED: none"));
    let by = |id: &str| json!({"claim_id": id});
    let fixtures = [
        fixture(by(&claim)),
        fixture(by("clm_404")),
        fixture(json!({})),
        fixture(by(&boom)),
        Fixture::with_ctx(broke(), by(&claim)),
        Fixture::with_ctx(other_project(), by(&claim)),
    ];
    contract::check(tool(&built, "agent_refute").as_ref(), "agent", &fixtures)
        .await
        .unwrap();
}
