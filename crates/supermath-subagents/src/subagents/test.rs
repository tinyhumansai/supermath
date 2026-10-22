//! Tests for assembling the five tools.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use supermath_tool::{Backend, Exposure, InMemoryCallLog, Kind, ToolRegistry};

use super::{Runners, Subagents, Unavailable};
use crate::child::ChildKind;
use crate::config::Models;
use crate::error::Error;
use crate::fake::{ScriptRunner, World, project, script, subagents, turn};
use crate::{ChildRunner, SubagentsConfig};

fn idle() -> Arc<ScriptRunner> {
    ScriptRunner::new(script(|_, _| async { Ok(turn("done")) }))
}

#[test]
fn a_full_registry_yields_all_five_tools_in_catalogue_order() {
    let world = World::new();
    let built = subagents(&world.registry(), &idle());
    let names: Vec<&str> = built.tools().iter().map(|t| t.name()).collect();
    assert_eq!(
        names,
        [
            "agent_digest_paper",
            "agent_formalize",
            "agent_review_alignment",
            "agent_refute",
            "agent_settle"
        ]
    );
    assert_eq!(built.unavailable().len(), 0);
    for tool in built.tools() {
        assert_eq!(tool.kind(), Kind::Agentic(Backend::OpenHuman));
        assert_eq!(tool.exposure(), Exposure::Deferred);
        assert!(tool.description().starts_with("Runs a"));
    }
    assert!(format!("{built:?}").contains("agent_refute"));
}

#[test]
fn a_tool_whose_primitives_are_missing_is_left_out_and_reported() {
    let world = World::new();
    let registry = world.registry_of(&[
        "lit_get_paper",
        "lit_extract_statements",
        "lit_citations",
        "ledger_record",
        "ledger_link",
        "recall_memory",
        "ledger_claims",
    ]);
    let built = subagents(&registry, &idle());
    let names: Vec<&str> = built.tools().iter().map(|t| t.name()).collect();
    assert_eq!(names, ["agent_digest_paper"]);
    let formalize = built
        .unavailable()
        .iter()
        .find(|u| u.tool == "agent_formalize")
        .unwrap();
    assert!(formalize.missing.contains(&"lean_check".to_owned()));
    assert!(!formalize.missing.contains(&"ledger_claims".to_owned()));
    let settle = built
        .unavailable()
        .iter()
        .find(|u| u.tool == "agent_settle")
        .unwrap();
    assert!(settle.missing.contains(&"py_search_counterexample".to_owned()));
    assert!(settle.missing.contains(&"lean_try_tactics".to_owned()));
    assert!(!settle.missing.iter().any(|name| name.starts_with("duel_")));
    assert_eq!(
        settle.missing.iter().filter(|name| *name == "lean_check").count(),
        1,
        "a primitive both sides need is listed once"
    );
    assert_eq!(built.unavailable().len(), 4);
}

#[test]
fn an_empty_registry_builds_nothing() {
    let built = subagents(&ToolRegistry::new(), &idle());
    assert_eq!(built.tools().len(), 0);
    let first: &Unavailable = &built.unavailable()[0];
    assert_eq!(first.tool, "agent_digest_paper");
}

#[tokio::test]
async fn an_agentic_primitive_would_let_children_nest_and_is_refused() {
    let world = World::new();
    let mut registry = world.registry_of(&[
        "lean_decl",
        "lean_search",
        "lit_get_paper",
        "recall_memory",
        "lean_check",
        "ledger_claims",
        "ledger_record",
    ]);
    // Rebuild `lean_decl` as an agentic tool: the child would call a model.
    let first = built_with_agentic("lean_decl");
    assert_eq!(first.schema()["additionalProperties"], false);
    assert_eq!(first.permission(), supermath_tool::Permission::Read);
    assert_eq!(first.errors().len(), 0);
    assert_eq!(first.description(), "Runs a model.");
    assert_eq!(first.exposure(), Exposure::Deferred);
    let answer = first
        .call(&crate::fake::ctx(), serde_json::Value::Null)
        .await;
    assert_eq!(answer.outcome, supermath_tool::Outcome::Ok);
    let mut swapped = ToolRegistry::new();
    for tool in registry.iter() {
        if tool.name() != "lean_decl" {
            swapped.register(Arc::clone(tool)).unwrap();
        }
    }
    swapped.register(first).unwrap();
    registry = swapped;
    let idle: Arc<dyn ChildRunner> = idle();
    let runners = Runners {
        reader: idle.clone(),
        formalizer: idle.clone(),
        reviewer: idle.clone(),
        refuter: idle.clone(),
        duel_prover: idle.clone(),
        duel_refuter: idle,
    };
    let error = Subagents::with_runners(
        &registry,
        Arc::new(InMemoryCallLog::new()),
        &project(),
        &runners,
        None,
    )
    .unwrap_err();
    assert!(matches!(error, Error::NotPrimitive(name) if name == "lean_decl"));
}

/// A stand-in tool named `name` that claims to be agentic.
fn built_with_agentic(name: &'static str) -> Arc<dyn supermath_tool::Tool> {
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use supermath_tool::{CallContext, Envelope, ErrorCode, Permission, Tool};

    struct Agentic(&'static str);
    #[async_trait]
    impl Tool for Agentic {
        fn name(&self) -> &'static str {
            self.0
        }
        fn description(&self) -> &'static str {
            "Runs a model."
        }
        fn schema(&self) -> Value {
            json!({"type": "object", "additionalProperties": false})
        }
        fn exposure(&self) -> Exposure {
            Exposure::Deferred
        }
        fn permission(&self) -> Permission {
            Permission::Read
        }
        fn kind(&self) -> Kind {
            Kind::Agentic(Backend::Gemini)
        }
        fn errors(&self) -> &'static [ErrorCode] {
            &[]
        }
        async fn call(&self, _: &CallContext, _: Value) -> Envelope {
            Envelope::ok(Value::Null)
        }
    }
    Arc::new(Agentic(name))
}

#[test]
fn registering_twice_reports_the_duplicate() {
    let world = World::new();
    let built = subagents(&world.registry(), &idle());
    let mut registry = ToolRegistry::new();
    built.register_into(&mut registry).unwrap();
    assert_eq!(registry.len(), 5);
    let error = built.register_into(&mut registry).unwrap_err();
    assert!(matches!(error, Error::Registry(_)));
}

#[test]
fn each_childs_scope_is_exactly_its_listed_primitives() {
    let world = World::new();
    let registry = world.registry();
    for kind in ChildKind::ALL {
        let spec = kind.spec();
        let scope = registry.subset(spec.primitives).unwrap();
        let mut listed: Vec<&str> = spec.primitives.to_vec();
        listed.sort_unstable();
        listed.dedup();
        assert_eq!(scope.names().collect::<Vec<_>>(), listed, "{}", spec.tool);
    }
}

#[test]
fn the_config_debug_hides_the_provider_internals_and_defaults_are_the_rungs() {
    let config = SubagentsConfig::default();
    assert_eq!(config.models, Models::default());
    let text = format!("{config:?}");
    assert!(text.contains("progress: false"), "{text}");
}

#[test]
fn an_agentic_primitive_only_a_duel_side_needs_is_refused() {
    // Without lit_get_paper only agent_refute and the duel are buildable;
    // agent_refute never calls lean_try_tactics, the duel prover does.
    let world = World::new();
    let names: Vec<&'static str> = crate::fake::world::PRIMITIVES
        .into_iter()
        .filter(|name| !matches!(*name, "lit_get_paper" | "lean_try_tactics"))
        .collect();
    let mut registry = world.registry_of(&names);
    registry.register(built_with_agentic("lean_try_tactics")).unwrap();
    let error = subagents_result(&registry).unwrap_err();
    assert!(matches!(error, Error::NotPrimitive(name) if name == "lean_try_tactics"));
}

fn subagents_result(registry: &ToolRegistry) -> crate::Result<Subagents> {
    let idle: Arc<dyn ChildRunner> = idle();
    let runners = Runners {
        reader: idle.clone(),
        formalizer: idle.clone(),
        reviewer: idle.clone(),
        refuter: idle.clone(),
        duel_prover: idle.clone(),
        duel_refuter: idle,
    };
    Subagents::with_runners(
        registry,
        Arc::new(InMemoryCallLog::new()),
        &project(),
        &runners,
        None,
    )
}
