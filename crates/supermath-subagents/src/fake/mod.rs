//! Fakes for this crate's unit tests: a world of fake primitives and a
//! scripted child runner.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod runner;
pub(crate) mod world;

pub(crate) use runner::{
    ControlRunner, ScriptRunner, controlled, script, turn, until_cancelled,
};
pub(crate) use world::World;

use std::sync::Arc;

use supermath_tool::{Budget, CallContext, InMemoryCallLog, Project, Tool, ToolRegistry};

use crate::{Runners, Subagents};

/// The project every test serves.
pub(crate) fn project() -> Project {
    Project::new("demo", "ws:test/project:demo").unwrap()
}

/// A call context with a budget big enough for any default turn.
pub(crate) fn ctx() -> CallContext {
    CallContext::new(project(), "call-1").with_budget(Budget {
        wall_time_ms: 3_600_000,
        ..Budget::agentic(100, 5_000_000)
    })
}

/// The four tools over `registry`, all driven by `runner`.
pub(crate) fn subagents(registry: &ToolRegistry, runner: &Arc<ScriptRunner>) -> Subagents {
    let as_dyn: Arc<dyn crate::ChildRunner> = runner.clone();
    let runners = Runners {
        reader: Arc::clone(&as_dyn),
        formalizer: Arc::clone(&as_dyn),
        reviewer: Arc::clone(&as_dyn),
        refuter: Arc::clone(&as_dyn),
        duel_prover: Arc::clone(&as_dyn),
        duel_refuter: as_dyn,
    };
    Subagents::with_runners(
        registry,
        Arc::new(InMemoryCallLog::new()),
        &project(),
        &runners,
        None,
    )
    .unwrap()
}

/// The tool called `name` from `subagents`.
pub(crate) fn tool(subagents: &Subagents, name: &str) -> Arc<dyn Tool> {
    subagents
        .tools()
        .iter()
        .find(|tool| tool.name() == name)
        .cloned()
        .unwrap_or_else(|| panic!("no tool {name}"))
}
