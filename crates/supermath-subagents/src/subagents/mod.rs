//! [`Subagents`]: the five `agent_*` tools, built for one project.
//!
//! The composition hands over a [`ToolRegistry`] holding the primitives by
//! name. For each child, `Subagents` takes the subset the child may call
//! (never an `agent_*` tool), builds the child as a top-level `OpenHuman`
//! agent, and wraps it in its tool. A tool whose primitives the registry
//! lacks is left out and listed in [`Subagents::unavailable`], so a
//! composition that has not wired the refutation tools yet still gets the
//! other three.

use std::sync::Arc;

use openhuman_embed::{Provider, Runtime};
use supermath_tool::{CallContext, CallLog, Project, Tool, ToolRegistry};

use crate::belt::{Belt, Progress};
use crate::child::{ChildKind, ChildRunner, ChildSpec, OpenHumanChild};
use crate::config::{DuelSettings, Models};
use crate::duel::Offline;
use crate::error::{Error, Result};
use crate::tools::{
    AgentDigestPaper, AgentFormalize, AgentRefute, AgentReviewAlignment, AgentSettle, Agentic,
    Shared,
};

/// How the children are built.
#[derive(Clone, Default)]
pub struct SubagentsConfig {
    /// The Surplus model of each child.
    pub models: Models,
    /// The provider the children run on; `None` keeps the runtime's own
    /// (Surplus Intelligence). Tests pass `Provider::custom` with a scripted model.
    pub provider: Option<Provider>,
    /// Receives each child's tool events, for the parent's progress stream.
    pub progress: Option<Arc<dyn Progress>>,
    /// How `agent_settle` paces and polices its two sides.
    pub duel: DuelSettings,
}

impl std::fmt::Debug for SubagentsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubagentsConfig")
            .field("models", &self.models)
            .field("provider", &self.provider)
            .field("progress", &self.progress.is_some())
            .field("duel", &self.duel)
            .finish()
    }
}

/// One runner per child, for hosts and tests that supply their own.
#[derive(Debug, Clone)]
pub struct Runners {
    /// Runs the paper reader.
    pub reader: Arc<dyn ChildRunner>,
    /// Runs the formalizer.
    pub formalizer: Arc<dyn ChildRunner>,
    /// Runs the alignment reviewer.
    pub reviewer: Arc<dyn ChildRunner>,
    /// Runs the refuter.
    pub refuter: Arc<dyn ChildRunner>,
    /// Runs the prover side of `agent_settle`.
    pub duel_prover: Arc<dyn ChildRunner>,
    /// Runs the refuter side of `agent_settle`.
    pub duel_refuter: Arc<dyn ChildRunner>,
}

impl Runners {
    fn for_kind(&self, kind: ChildKind) -> Arc<dyn ChildRunner> {
        match kind {
            ChildKind::Reader => Arc::clone(&self.reader),
            ChildKind::Formalizer => Arc::clone(&self.formalizer),
            ChildKind::Reviewer => Arc::clone(&self.reviewer),
            ChildKind::Refuter => Arc::clone(&self.refuter),
            ChildKind::DuelProver => Arc::clone(&self.duel_prover),
            ChildKind::DuelRefuter => Arc::clone(&self.duel_refuter),
        }
    }
}

/// An `agent_*` tool left out because the registry lacks primitives it needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unavailable {
    /// The tool that was left out.
    pub tool: &'static str,
    /// The registry tools it needs and does not find.
    pub missing: Vec<String>,
}

/// The `agent_*` tools of one project.
pub struct Subagents {
    tools: Vec<Arc<dyn Tool>>,
    unavailable: Vec<Unavailable>,
}

impl std::fmt::Debug for Subagents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subagents")
            .field(
                "tools",
                &self.tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
            )
            .field("unavailable", &self.unavailable)
            .finish()
    }
}

impl Subagents {
    /// Builds the children on `runtime` and the tools that drive them.
    ///
    /// `registry` holds the primitives; `log` is the call log every child and
    /// verification call is recorded in (pass the one the ledger reads, so a
    /// `tool` source resolves); `project` is the project these tools serve.
    ///
    /// # Errors
    ///
    /// [`Error::Agent`] when `OpenHuman` refuses a child;
    /// [`Error::NotPrimitive`] when a primitive in the registry is agentic.
    pub fn new(
        runtime: &Runtime,
        registry: &ToolRegistry,
        log: Arc<dyn CallLog>,
        project: &Project,
        config: &SubagentsConfig,
    ) -> Result<Self> {
        Self::assemble(
            registry,
            log,
            project,
            (config.progress.clone(), config.duel),
            |spec, build_belt| {
                let model = match spec.kind {
                    ChildKind::Reader => &config.models.reader,
                    ChildKind::Formalizer | ChildKind::Refuter => &config.models.formalizer,
                    ChildKind::Reviewer => &config.models.reviewer,
                    ChildKind::DuelProver | ChildKind::DuelRefuter => &config.models.duel,
                };
                let child = OpenHumanChild::build(
                    runtime,
                    project,
                    spec,
                    model,
                    config.provider.as_ref(),
                    build_belt,
                )?;
                Ok(Arc::new(child) as Arc<dyn ChildRunner>)
            },
        )
    }

    /// Builds the tools over runners the host supplies instead of `OpenHuman`
    /// agents.
    ///
    /// # Errors
    ///
    /// [`Error::NotPrimitive`] when a primitive in the registry is agentic.
    pub fn with_runners(
        registry: &ToolRegistry,
        log: Arc<dyn CallLog>,
        project: &Project,
        runners: &Runners,
        progress: Option<Arc<dyn Progress>>,
    ) -> Result<Self> {
        Self::assemble(
            registry,
            log,
            project,
            (progress, DuelSettings::default()),
            |spec, _| Ok(runners.for_kind(spec.kind)),
        )
    }

    fn assemble(
        registry: &ToolRegistry,
        log: Arc<dyn CallLog>,
        project: &Project,
        (progress, duel): (Option<Arc<dyn Progress>>, DuelSettings),
        mut make: impl FnMut(&ChildSpec, &Arc<Belt>) -> Result<Arc<dyn ChildRunner>>,
    ) -> Result<Self> {
        let shared = Arc::new(Shared::new(
            registry.clone(),
            log,
            project.clone(),
            progress,
        ));
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
        let mut unavailable = Vec::new();
        for kind in ChildKind::ALL {
            let spec = kind.spec();
            let missing: Vec<String> = spec
                .required()
                .filter(|name| registry.get(name).is_none())
                .map(str::to_owned)
                .collect();
            if !missing.is_empty() {
                unavailable.push(Unavailable {
                    tool: spec.tool,
                    missing,
                });
                continue;
            }
            if let Some(agentic) = spec
                .required()
                .find(|name| registry.get(name).is_some_and(|t| t.kind().is_agentic()))
            {
                return Err(Error::NotPrimitive(agentic.to_owned()));
            }
            let scope = registry.subset(spec.primitives)?;
            let build_belt = Arc::new(Belt::new(
                spec.tool,
                scope.clone(),
                Arc::clone(&shared.log),
                CallContext::new(project.clone(), "subagents-build"),
                shared.progress.clone(),
            ));
            let runner = make(&spec, &build_belt)?;
            let core = Agentic::new(Arc::clone(&shared), spec, runner, scope);
            tools.push(match kind {
                ChildKind::Reader => Arc::new(AgentDigestPaper { core }),
                ChildKind::Formalizer => Arc::new(AgentFormalize { core }),
                ChildKind::Reviewer => Arc::new(AgentReviewAlignment { core }),
                ChildKind::Refuter => Arc::new(AgentRefute { core }),
                ChildKind::DuelProver | ChildKind::DuelRefuter => continue,
            });
        }
        match Self::duel(registry, project, &shared, &mut make)? {
            Ok([prover, refuter]) => tools.push(Arc::new(AgentSettle {
                prover,
                refuter,
                settings: duel,
            })),
            Err(missing) => unavailable.push(missing),
        }
        Ok(Self { tools, unavailable })
    }

    /// Builds the two sides of `agent_settle`, or says which primitives the
    /// registry lacks for them.
    fn duel(
        registry: &ToolRegistry,
        project: &Project,
        shared: &Arc<Shared>,
        make: &mut impl FnMut(&ChildSpec, &Arc<Belt>) -> Result<Arc<dyn ChildRunner>>,
    ) -> Result<std::result::Result<[Arc<Agentic>; 2], Unavailable>> {
        let mut missing: Vec<String> = Vec::new();
        for kind in ChildKind::DUEL {
            for name in kind.spec().required() {
                if registry.get(name).is_none() && !missing.iter().any(|m| m == name) {
                    missing.push(name.to_owned());
                }
            }
        }
        if !missing.is_empty() {
            return Ok(Err(Unavailable {
                tool: "agent_settle",
                missing,
            }));
        }
        let mut build = |kind: ChildKind| -> Result<Arc<Agentic>> {
            let spec = kind.spec();
            if let Some(agentic) = spec
                .required()
                .find(|name| registry.get(name).is_some_and(|t| t.kind().is_agentic()))
            {
                return Err(Error::NotPrimitive(agentic.to_owned()));
            }
            let primitives: Vec<&str> = spec
                .primitives
                .iter()
                .copied()
                .filter(|name| !name.starts_with("duel_"))
                .collect();
            let scope = registry.subset(&primitives)?;
            // The agent is built with stand-ins for the duel's own tools;
            // every turn replaces them with the ones bound to its duel.
            let mut offered = scope.clone();
            offered.register(Arc::new(Offline("duel_post")))?;
            offered.register(Arc::new(Offline("duel_board")))?;
            let build_belt = Arc::new(Belt::new(
                spec.tool,
                offered,
                Arc::clone(&shared.log),
                CallContext::new(project.clone(), "subagents-build"),
                shared.progress.clone(),
            ));
            let runner = make(&spec, &build_belt)?;
            Ok(Arc::new(Agentic::new(
                Arc::clone(shared),
                spec,
                runner,
                scope,
            )))
        };
        Ok(Ok([
            build(ChildKind::DuelProver)?,
            build(ChildKind::DuelRefuter)?,
        ]))
    }

    /// The tools that could be built, in catalogue order.
    #[must_use]
    pub fn tools(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }

    /// The tools left out, and the registry tools each needs.
    #[must_use]
    pub fn unavailable(&self) -> &[Unavailable] {
        &self.unavailable
    }

    /// Adds every built tool to `registry`.
    ///
    /// # Errors
    ///
    /// [`Error::Registry`] when a tool with the same name is already there.
    pub fn register_into(&self, registry: &mut ToolRegistry) -> Result<()> {
        for tool in &self.tools {
            registry.register(Arc::clone(tool))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test;
