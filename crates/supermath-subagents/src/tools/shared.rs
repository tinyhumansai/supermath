//! What the `agent_*` tools share: the registry, the child, admission,
//! and one way to run a turn with accounting.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use serde::de::DeserializeOwned;
use serde_json::Value;
use supermath_tool::{CallContext, CallLog, Envelope, ErrorCode, Project, ToolRegistry, Usage};

use crate::belt::{Belt, Progress};
use crate::child::{ChildEnd, ChildRequest, ChildRunner, ChildSpec, TurnControl};
use crate::limits::{BudgetArg, Limits};

/// An error envelope on the way out of a helper; boxed so `Result` stays small.
pub(crate) type Refusal = Box<Envelope>;

/// The parts every tool of one `Subagents` holds in common.
pub(crate) struct Shared {
    pub(crate) registry: ToolRegistry,
    pub(crate) log: Arc<dyn CallLog>,
    pub(crate) project: Project,
    pub(crate) progress: Option<Arc<dyn Progress>>,
    verifications: AtomicU32,
}

impl Shared {
    pub(crate) fn new(
        registry: ToolRegistry,
        log: Arc<dyn CallLog>,
        project: Project,
        progress: Option<Arc<dyn Progress>>,
    ) -> Self {
        Self {
            registry,
            log,
            project,
            progress,
            verifications: AtomicU32::new(0),
        }
    }
}

/// One `agent_*` tool's child and its place in the registry.
pub(crate) struct Agentic {
    pub(crate) shared: Arc<Shared>,
    pub(crate) spec: ChildSpec,
    pub(crate) runner: Arc<dyn ChildRunner>,
    scope: ToolRegistry,
}

/// A finished turn, with the belt that journaled its calls.
pub(crate) struct Turn {
    pub(crate) reply: String,
    pub(crate) session_id: String,
    pub(crate) end: ChildEnd,
    pub(crate) usage: Usage,
    pub(crate) belt: Arc<Belt>,
    /// Steers the turn never applied, for the host's next turn.
    pub(crate) unapplied: Vec<String>,
}

impl Agentic {
    pub(crate) fn new(
        shared: Arc<Shared>,
        spec: ChildSpec,
        runner: Arc<dyn ChildRunner>,
        scope: ToolRegistry,
    ) -> Self {
        Self {
            shared,
            spec,
            runner,
            scope,
        }
    }

    /// Usage for an answer that ran no turn.
    pub(crate) fn zero_usage(&self) -> Usage {
        Usage {
            model: self.runner.model().to_owned(),
            ..Usage::default()
        }
    }

    /// `envelope` with `usage` filled in when the tool did not set any.
    pub(crate) fn finish(&self, mut envelope: Envelope) -> Envelope {
        if envelope.usage.is_none() {
            envelope.usage = Some(self.zero_usage());
        }
        envelope
    }

    /// Whether the registry holds `tool`.
    pub(crate) fn has(&self, tool: &str) -> bool {
        self.shared.registry.get(tool).is_some()
    }

    /// Calls a primitive as the tool itself (not as the child), logged.
    pub(crate) async fn call(&self, ctx: &CallContext, tool: &str, args: Value) -> Envelope {
        let number = self.shared.verifications.fetch_add(1, Ordering::SeqCst) + 1;
        let mut ctx = ctx.clone();
        ctx.call_id = format!("{}.verify.{tool}.{number}", ctx.call_id);
        self.shared
            .registry
            .dispatch(self.shared.log.as_ref(), tool, &ctx, args)
            .await
    }

    /// Refuses a call made in another project than the one this tool serves.
    pub(crate) fn check_project(&self, ctx: &CallContext) -> Result<(), Refusal> {
        if ctx.project == self.shared.project {
            Ok(())
        } else {
            Err(Box::new(Envelope::error(
                ErrorCode::Forbidden,
                format!(
                    "this tool serves project {}, not {}",
                    self.shared.project.slug(),
                    ctx.project.slug()
                ),
            )))
        }
    }

    /// Resolves the call's limits and admits them against the caller's budget.
    pub(crate) fn admit(
        ctx: &CallContext,
        base: Limits,
        arg: Option<BudgetArg>,
    ) -> Result<Limits, Refusal> {
        let limits = base
            .apply(arg.unwrap_or_default())
            .map_err(|message| Box::new(Envelope::error(ErrorCode::InvalidInput, message)))?;
        ctx.budget
            .admit(&limits.demand())
            .map_err(|refusal| Box::new(Envelope::from(refusal)))?;
        Ok(limits)
    }

    /// Runs the child for one call and returns what its belt journaled.
    ///
    /// A budget that cut the turn short is [`ChildEnd::OutOfBudget`], not an
    /// error; a backend failure is a `BACKEND_UNAVAILABLE` envelope.
    pub(crate) async fn run(
        &self,
        ctx: &CallContext,
        prompt: String,
        limits: Limits,
    ) -> Result<Turn, Refusal> {
        let belt = Arc::new(self.belt(ctx, self.scope.clone()));
        self.run_on(ctx, prompt, limits, belt, None).await
    }

    /// A belt for this tool's child under `ctx`, reaching the tools in
    /// `scope`.
    pub(crate) fn belt(&self, ctx: &CallContext, scope: ToolRegistry) -> Belt {
        Belt::new(
            self.spec.tool,
            scope,
            Arc::clone(&self.shared.log),
            ctx.clone(),
            self.shared.progress.clone(),
        )
    }

    /// The primitives this tool's child may call.
    pub(crate) fn scope(&self) -> &ToolRegistry {
        &self.scope
    }

    /// Runs one turn of the child on `belt`, steered or cancelled through
    /// `control` when the host holds one. Ends as [`Agentic::run`] does.
    pub(crate) async fn run_on(
        &self,
        ctx: &CallContext,
        prompt: String,
        limits: Limits,
        belt: Arc<Belt>,
        control: Option<TurnControl>,
    ) -> Result<Turn, Refusal> {
        let request = ChildRequest {
            prompt,
            limits,
            belt: Arc::clone(&belt),
            control,
        };
        let started = Instant::now();
        let outcome = tokio::select! {
            result = tokio::time::timeout(limits.wall(), self.runner.run(request)) => result.ok(),
            () = ctx.cancel.cancelled() => {
                return Ok(self.turn_from(None, &belt, started, ChildEnd::OutOfBudget("cancelled".into())));
            }
        };
        match outcome {
            None => Ok(self.turn_from(
                None,
                &belt,
                started,
                ChildEnd::OutOfBudget("wall time".into()),
            )),
            Some(Err(message)) => {
                let usage = self.usage_of(None, &belt, started);
                Err(Box::new(
                    Envelope::error(
                        ErrorCode::BackendUnavailable,
                        format!("the child's model backend failed: {message}"),
                    )
                    .with_usage(usage),
                ))
            }
            Some(Ok(turn)) => {
                let end = turn.end.clone();
                Ok(self.turn_from(Some(turn), &belt, started, end))
            }
        }
    }

    fn turn_from(
        &self,
        turn: Option<crate::child::ChildTurn>,
        belt: &Arc<Belt>,
        started: Instant,
        end: ChildEnd,
    ) -> Turn {
        let usage = self.usage_of(turn.as_ref(), belt, started);
        let (reply, session_id, unapplied) = turn.map_or_else(Default::default, |t| {
            (t.reply, t.session_id, t.unapplied)
        });
        Turn {
            reply,
            session_id,
            end,
            usage,
            belt: Arc::clone(belt),
            unapplied,
        }
    }

    fn usage_of(
        &self,
        turn: Option<&crate::child::ChildTurn>,
        belt: &Belt,
        started: Instant,
    ) -> Usage {
        Usage {
            model: self.runner.model().to_owned(),
            turns: turn.map_or(0, |t| t.turns),
            tool_calls: belt.tool_calls(),
            input_tokens: turn.map_or(0, |t| t.input_tokens),
            output_tokens: turn.map_or(0, |t| t.output_tokens),
            elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        }
    }
}

/// Decodes a tool's arguments, refusing unknown or mistyped fields.
pub(crate) fn parse_args<T: DeserializeOwned>(args: Value) -> Result<T, Refusal> {
    serde_json::from_value(args).map_err(|error| {
        Box::new(Envelope::error(
            ErrorCode::InvalidInput,
            format!("bad arguments: {error}"),
        ))
    })
}

/// At most `n` whitespace-separated words of `text`, rejoined with spaces.
pub(crate) fn words(text: &str, n: usize) -> String {
    text.split_whitespace()
        .take(n)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The reason a budget cut a turn short, as an `unknowns` entry.
pub(crate) fn out_of_budget(end: &ChildEnd) -> Option<String> {
    match end {
        ChildEnd::Finished => None,
        ChildEnd::OutOfBudget(why) => Some(format!("the child ran out of budget ({why})")),
    }
}
