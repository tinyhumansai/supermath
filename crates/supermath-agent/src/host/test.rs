//! Unit tests for the host adapter: the adapted tools match the composed
//! toolkit, and calls land in the shared call log.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use openhuman_embed::{PermissionLevel, Tool as _, ToolExposure, ToolPolicy};
use serde_json::{Value, json};
use supermath_mcp::{Config, Toolkit};
use supermath_subagents::{ChildRequest, ChildRunner, ChildTurn, Runners, Subagents};
use supermath_tool::contract::description_violation;
use supermath_tool::{Exposure, Outcome, Permission, Tool};

use super::Host;
use crate::error::Error;

/// A child runner that never runs a turn.
#[derive(Debug)]
struct Idle;

#[async_trait::async_trait]
impl ChildRunner for Idle {
    fn model(&self) -> &'static str {
        "idle"
    }
    async fn run(&self, _: ChildRequest) -> Result<ChildTurn, String> {
        Err("idle".into())
    }
}

fn toolkit() -> Arc<Toolkit> {
    let config = Config::from_lookup(|_| None).unwrap();
    Arc::new(Toolkit::compose(&config, "odd-sums").unwrap())
}

/// The five `agent_*` tools, over runners that never run.
fn agent_tools(toolkit: &Toolkit) -> Vec<Arc<dyn Tool>> {
    let idle: Arc<dyn ChildRunner> = Arc::new(Idle);
    let runners = Runners {
        reader: idle.clone(),
        formalizer: idle.clone(),
        reviewer: idle.clone(),
        refuter: idle.clone(),
        duel_prover: idle.clone(),
        duel_refuter: idle,
    };
    let built = Subagents::with_runners(
        &toolkit.registry(),
        toolkit.call_log(),
        toolkit.project(),
        &runners,
        None,
    )
    .unwrap();
    assert_eq!(built.unavailable(), []);
    built.tools().to_vec()
}

fn host() -> Arc<Host> {
    let toolkit = toolkit();
    let agents = agent_tools(&toolkit);
    Arc::new(Host::new(toolkit, &agents).unwrap())
}

/// The envelope a result carries, whether it was stored as JSON or text.
fn body(result: &openhuman_embed::ToolResult) -> Value {
    let first = serde_json::to_value(result).unwrap()["content"][0].clone();
    match first.get("data") {
        Some(data) => data.clone(),
        None => serde_json::from_str(first["text"].as_str().unwrap()).unwrap(),
    }
}

#[test]
fn every_adapted_tool_matches_its_toolkit_tool() {
    let host = host();
    let adapted = host.adapt();
    let mut expected: Vec<Arc<dyn Tool>> = host.toolkit().registry().iter().cloned().collect();
    expected.extend(host.agents.iter().cloned());
    assert_eq!(adapted.len(), expected.len());

    for (host_tool, tool) in adapted.iter().zip(&expected) {
        assert_eq!(host_tool.name(), tool.name());
        assert_eq!(host_tool.description(), tool.description());
        assert_eq!(
            host_tool.parameters_schema(),
            tool.schema(),
            "{}",
            tool.name()
        );
        assert_eq!(host_tool.kind(), tool.kind(), "{}", tool.name());
        assert_eq!(
            host_tool.exposure(),
            match tool.exposure() {
                Exposure::Direct => ToolExposure::Direct,
                Exposure::Deferred => ToolExposure::Deferred,
            },
            "{}",
            tool.name()
        );
        match tool.permission() {
            Permission::Read => {
                assert_eq!(host_tool.permission_level(), PermissionLevel::ReadOnly);
                let mut expected = ToolPolicy::read_only();
                if tool.kind().is_agentic() {
                    expected.runtime.timeout_ms = Some(3_600_000);
                }
                assert_eq!(host_tool.policy(), expected, "{}", tool.name());
            }
            Permission::Write => {
                assert_eq!(host_tool.permission_level(), PermissionLevel::Write);
                assert!(
                    host_tool.policy().side_effects.writes_files,
                    "{}",
                    tool.name()
                );
            }
        }
    }
}

#[test]
fn the_catalogue_has_reads_writes_and_both_exposures() {
    let host = host();
    let adapted = host.adapt();
    assert!(
        adapted
            .iter()
            .any(|t| t.permission_level() == PermissionLevel::Write)
    );
    assert!(
        adapted
            .iter()
            .any(|t| t.permission_level() == PermissionLevel::ReadOnly)
    );
    assert!(adapted.iter().any(|t| t.exposure() == ToolExposure::Direct));
    assert!(
        adapted
            .iter()
            .any(|t| t.exposure() == ToolExposure::Deferred)
    );
    assert!(adapted.iter().any(|t| t.kind().is_agentic()));
}

#[test]
fn tool_names_list_the_toolkit_then_the_agent_tools() {
    let host = host();
    let names = host.tool_names();
    assert_eq!(names.len(), host.adapt().len());
    assert!(names.contains(&"calc_factor"));
    assert!(names.contains(&"recall_memory"));
    let first_agent = names.iter().position(|n| n.starts_with("agent_")).unwrap();
    assert!(names[first_agent..].iter().all(|n| n.starts_with("agent_")));
    assert_eq!(names.len() - first_agent, 5);
}

#[tokio::test]
async fn a_primitive_call_returns_the_envelope_with_its_call_id_and_is_logged() {
    let host = host();
    let tool = host
        .adapt()
        .into_iter()
        .find(|t| t.name() == "calc_factor")
        .unwrap();
    let result = tool.execute(json!({"n": 360})).await.unwrap();
    assert!(!result.is_error);
    let body = body(&result);
    assert_eq!(body["call_id"], "call-1");
    assert_eq!(body["outcome"], "ok");

    let record = host.toolkit().call_log().get("call-1").unwrap();
    assert_eq!(record.tool, "calc_factor");
    assert_eq!(record.outcome, Outcome::Ok);
}

#[tokio::test]
async fn a_refusal_envelope_is_an_answer_the_turn_continues_past() {
    // OpenHuman halts a turn on a failed call it classifies as a credential
    // or uncertain-effect failure, and it classifies a refusal's text by
    // keyword: `FORBIDDEN` read as "credentials rejected". A refusal is the
    // tool's answer, so it reaches the model as a result, still `error`.
    let host = host();
    let tool = host
        .adapt()
        .into_iter()
        .find(|t| t.name() == "calc_factor")
        .unwrap();
    let result = tool.execute(json!({"n": "ten"})).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(body(&result)["outcome"], "error");
    assert_eq!(body(&result)["error"]["code"], "INVALID_INPUT");
}

#[tokio::test]
async fn an_agent_tool_is_dispatched_by_the_host_under_its_own_call_id() {
    let host = host();
    let done = host
        .dispatch(
            "agent_formalize",
            json!({"source": {"statement": "2 + 2 = 4"}, "path": "lean/Supermath/A.lean"}),
        )
        .await;
    assert_eq!(done.call_id, "agent-1");
    let record = host.toolkit().call_log().get("agent-1").unwrap();
    assert_eq!(record.tool, "agent_formalize");
    // The next toolkit call keeps the toolkit's own numbering.
    assert_eq!(
        host.dispatch("calc_factor", json!({"n": 6})).await.call_id,
        "call-1"
    );
}

#[tokio::test]
async fn an_unknown_tool_answers_not_found() {
    let host = host();
    let done = host.dispatch("no_such_tool", json!({})).await;
    assert_eq!(
        done.envelope.code(),
        Some(supermath_tool::ErrorCode::NotFound)
    );
}

#[test]
fn rejects_an_agent_tool_that_shadows_a_toolkit_tool() {
    let toolkit = toolkit();
    let shadow = toolkit.registry().get("calc_factor").unwrap().clone();
    let error = Host::new(toolkit, &[shadow]).unwrap_err();
    assert!(matches!(error, Error::DuplicateTool(name) if name == "calc_factor"));
}

#[test]
fn rejects_two_agent_tools_with_one_name() {
    let toolkit = toolkit();
    let agents = agent_tools(&toolkit);
    let doubled = vec![agents[0].clone(), agents[0].clone()];
    let error = Host::new(toolkit, &doubled).unwrap_err();
    assert!(matches!(error, Error::DuplicateTool(_)), "{error}");
}

#[test]
fn the_belt_boxes_every_adapted_tool_and_debug_names_a_tool() {
    let host = host();
    assert_eq!(host.belt().len(), host.tool_names().len());
    let first = host.adapt().remove(0);
    assert_eq!(format!("{first:?}"), "HostTool(\"calc_factor\")");
}

#[test]
fn agent_tools_ask_for_their_own_wall_time_and_primitives_inherit_the_default() {
    // OpenHuman otherwise cuts every call at its 120 s default: a duel with
    // a 30-minute budget was killed after two minutes.
    let host = host();
    for tool in host.adapt() {
        let asked = tool.policy().runtime.timeout_ms;
        // `web_research` is agentic too: it runs its own 180 s deadline,
        // which the 120 s default cut short.
        if tool.kind().is_agentic() {
            assert_eq!(asked, Some(3_600_000), "{}", tool.name());
        } else {
            assert_eq!(asked, None, "{}", tool.name());
        }
    }
}

#[test]
fn the_runners_catalogue_has_exactly_six_direct_tools_and_question_descriptions() {
    let host = host();
    let adapted = host.adapt();
    let mut direct: Vec<&str> = adapted
        .iter()
        .filter(|tool| tool.exposure() == ToolExposure::Direct)
        .map(openhuman_embed::Tool::name)
        .collect();
    direct.sort_unstable();
    assert_eq!(
        direct,
        [
            "lean_check",
            "ledger_record",
            "lit_search_arxiv",
            "py_run",
            "recall_memory",
            "web_search"
        ]
    );
    assert!(adapted.iter().any(|tool| tool.name() == "agent_settle"));
    let tools = host.toolkit().registry();
    for tool in tools.iter().chain(host.agents.iter()) {
        assert_eq!(
            description_violation(tool.as_ref()),
            None,
            "{}",
            tool.name()
        );
    }
}
