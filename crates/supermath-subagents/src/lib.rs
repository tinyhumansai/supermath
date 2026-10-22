//! The `agent_*` tools: child `OpenHuman` agents behind the supermath tool
//! interface.
//!
//! Reading a paper or formalizing a theorem takes dozens of tool calls. Done
//! in the top-level agent they would fill its context with Lean errors and
//! LaTeX. Each tool here hands one such task to a **child agent** with its own
//! prompt, its own primitives, and its own model, and returns only the
//! outcome:
//!
//! | Tool | Child model | Returns |
//! | --- | --- | --- |
//! | [`AgentDigestPaper`] | `SUPERMATH_READER_MODEL` (`deepseek-v4.1-flash`) | ledger ids recorded from a paper |
//! | [`AgentFormalize`] | `SUPERMATH_FORMALIZER_MODEL` (`deepseek-v4-pro`) | Lean files with verdicts |
//! | [`AgentReviewAlignment`] | `SUPERMATH_REVIEWER_MODEL` (`glm-5.3`) | `accepted` or `disputed` |
//! | [`AgentRefute`] | `SUPERMATH_FORMALIZER_MODEL` | a Lean-confirmed counterexample, or the frames searched |
//! | [`AgentSettle`] | `SUPERMATH_DUEL_MODEL` (`deepseek-v4-pro`), two children | `proved`, `refuted`, or `unknown` with the bounds searched |
//!
//! [`AgentSettle`] is the one tool with two children. A prover and a refuter
//! run in parallel on one pinned conjecture, with equal budgets, in one
//! workspace where each writes only to its own area. They exchange posts the
//! tool checks before relaying, the tool nudges or restarts a side that
//! stops making progress, and the first theorem it verifies against the
//! pinned statement cancels the other side
//! (`docs/specs/toolkit/subagents/tools/agent_settle.md`).
//!
//! # The rule that makes them trustworthy
//!
//! A child's reply is never a verdict. After the turn, each tool re-runs the
//! primitive that decides its answer (`lean_check`, `ledger_claims`,
//! `file_read`) through the same [`ToolRegistry`] and reports that. What a
//! child wrote comes from the registry's own answers to its `file_write`
//! calls, not from its account. Children hold primitives only, so they cannot
//! spawn children, and a child that runs out of budget yields
//! `outcome: "unknown"`, never `failed`.
//!
//! # Entry points
//!
//! - [`Subagents::new`] builds the children on an `OpenHuman` [`Runtime`] and
//!   the tools that drive them, for one project.
//! - [`Subagents::register_into`] adds the tools to a registry.
//! - [`Models`] reads the four model variables, and [`DuelSettings`] how
//!   `agent_settle` paces its sides.
//! - [`TurnControl`] steers or cancels a running child turn.
//! - [`ChildRunner`] is the port tests and hosts substitute for `OpenHuman`.
//!
//! # Example
//!
//! Tools built over a scripted runner and an empty registry report which
//! primitives they are missing:
//!
//! ```
//! use std::sync::Arc;
//! use supermath_subagents::{Runners, Subagents};
//! use supermath_tool::{InMemoryCallLog, Project, ToolRegistry};
//! # use supermath_subagents::{ChildRequest, ChildRunner, ChildTurn};
//! # #[derive(Debug)] struct Idle;
//! # #[async_trait::async_trait]
//! # impl ChildRunner for Idle {
//! #     fn model(&self) -> &str { "idle" }
//! #     async fn run(&self, _: ChildRequest) -> Result<ChildTurn, String> { Err("idle".into()) }
//! # }
//! let idle: Arc<dyn ChildRunner> = Arc::new(Idle);
//! let runners = Runners {
//!     reader: idle.clone(), formalizer: idle.clone(), reviewer: idle.clone(),
//!     refuter: idle.clone(), duel_prover: idle.clone(), duel_refuter: idle,
//! };
//! let project = Project::new("demo", "ws:demo/project:demo").unwrap();
//! let subagents = Subagents::with_runners(
//!     &ToolRegistry::new(), Arc::new(InMemoryCallLog::new()), &project, &runners, None,
//! ).unwrap();
//! assert!(subagents.tools().is_empty());
//! assert_eq!(subagents.unavailable().len(), 5);
//! ```
//!
//! # What this crate does not hold
//!
//! No primitive and no domain crate: it reaches `lean_check`, `ledger_record`
//! and the rest only through the [`ToolRegistry`] the composition hands it,
//! which keeps it testable with fakes and keeps "children use primitives
//! only" a property of what the registry holds. No model choice beyond the
//! four Surplus model names in [`Models`], which the environment sets.

mod belt;
pub mod bench;
mod child;
mod config;
mod duel;
mod error;
mod limits;
mod subagents;
mod tools;

pub use belt::{Belt, CallNote, HostTool, Progress, tool_result};
pub use child::{
    ChildEnd, ChildKind, ChildRequest, ChildRunner, ChildSpec, ChildTurn, ControlSignal,
    OpenHumanChild, TurnControl,
};
pub use config::{DuelSettings, Models};
pub use error::{Error, Result};
pub use limits::{BudgetArg, Limits, budget_schema};
pub use subagents::{Runners, Subagents, SubagentsConfig, Unavailable};
pub use tools::{AgentDigestPaper, AgentFormalize, AgentRefute, AgentReviewAlignment, AgentSettle};

#[cfg(doc)]
use openhuman_embed::Runtime;
#[cfg(doc)]
use supermath_tool::ToolRegistry;

#[cfg(test)]
mod fake;
