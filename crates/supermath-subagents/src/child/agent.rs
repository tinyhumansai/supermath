//! [`OpenHumanChild`]: a child as a top-level `OpenHuman` agent.

use std::sync::{Arc, Mutex, PoisonError};

use openhuman_embed::seams::{StopDecision, StopHook, TurnCost, TurnState};
use openhuman_embed::{
    Access, Agent, AgentDefinitionSpec, AgentSpec, CoreError, HostTurnTools, Provider, Runtime,
    ToolScopeSpec, TurnCancellation, TurnSteering,
};
use supermath_tool::Project;

use super::{
    ChildEnd, ChildRequest, ChildRunner, ChildSpec, ChildTurn, ControlSignal, TurnControl,
};
use crate::belt::Belt;
use crate::error::{Error, Result};
use crate::limits::Limits;

/// The most model calls any child agent is built for. A call's own limit is
/// enforced per turn by a stop hook, so one agent serves every budget.
const BUILT_ITERATIONS: usize = 64;

/// A child agent built ahead of time and run one turn per call.
#[derive(Debug, Clone)]
pub struct OpenHumanChild {
    agent: Agent,
    model: String,
}

impl OpenHumanChild {
    /// Registers the child described by `spec` for `project` on `runtime`.
    ///
    /// The agent's tool scope is `Named` with exactly the spec's primitives
    /// and `memory` is disallowed, so `OpenHuman`'s own memory tool does not
    /// shadow the ledger. `build_belt` supplies the tools the agent is built
    /// with; every turn replaces them with a belt bound to that call.
    ///
    /// # Errors
    ///
    /// [`Error::Agent`] when `OpenHuman` refuses the agent, for example a
    /// project slug too long for an agent id.
    pub fn build(
        runtime: &Runtime,
        project: &Project,
        spec: &ChildSpec,
        model: &str,
        provider: Option<&Provider>,
        build_belt: &Arc<Belt>,
    ) -> Result<Self> {
        let id = spec.agent_id(project);
        let belt = Arc::clone(build_belt);
        let mut agent_spec = AgentSpec::new(id.clone())
            // Acts without parking: the scope below is what bounds the child.
            .access(Access::full())
            .definition(
                AgentDefinitionSpec::new()
                    .bare_prompt(spec.prompt)
                    .tools(ToolScopeSpec::Named(
                        spec.primitives
                            .iter()
                            .map(|name| (*name).to_owned())
                            .collect(),
                    ))
                    .disallow_tools(["memory"])
                    .max_iterations(BUILT_ITERATIONS),
            )
            .tools(move |_| HostTurnTools::advertised(belt.host_tools()));
        if let Some(provider) = provider {
            agent_spec = agent_spec.provider(provider.clone());
        }
        let agent = runtime
            .agent(agent_spec.model(model))
            .map_err(|error| Error::Agent {
                id,
                detail: error.to_string(),
            })?;
        Ok(Self {
            agent,
            model: model.to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl ChildRunner for OpenHumanChild {
    fn model(&self) -> &str {
        &self.model
    }

    async fn run(&self, request: ChildRequest) -> std::result::Result<ChildTurn, String> {
        let ChildRequest {
            prompt,
            session_id,
            limits,
            belt,
            control,
        } = request;
        let hook = Arc::new(LimitHook::new(limits, Arc::clone(&belt)));
        let turn_belt = Arc::clone(&belt);
        let mut turn = self
            .agent
            .turn(prompt)
            .session(session_id.clone())
            .tools(move |_| HostTurnTools::advertised(turn_belt.host_tools()))
            .stop_hook(hook.clone())
            .timeout(limits.wall());
        let (sent, steering) = match control {
            None => (turn.send().await, None),
            Some(control) => {
                let steering = turn.steering_handle();
                let cancellation = turn.cancellation_handle();
                let sent = drive(turn.send(), &control, &steering, cancellation).await;
                (sent, Some(steering))
            }
        };
        let tally = hook.tally();
        match sent {
            Err(CoreError::TurnCancelled { .. }) => Ok(ChildTurn {
                reply: String::new(),
                session_id,
                turns: tally.iterations,
                input_tokens: tally.input,
                output_tokens: tally.output,
                end: ChildEnd::OutOfBudget("cancelled".into()),
                unapplied: steering.map(|s| s.take_unapplied()).unwrap_or_default(),
            }),
            Ok(outcome) => {
                let (input_tokens, output_tokens) = outcome
                    .usage
                    .as_ref()
                    .map_or((tally.input, tally.output), |usage| {
                        (usage.input_tokens, usage.output_tokens)
                    });
                Ok(ChildTurn {
                    reply: outcome.reply,
                    session_id: outcome.session_id,
                    turns: tally.iterations,
                    input_tokens,
                    output_tokens,
                    end: hook
                        .cut_off()
                        .map_or(ChildEnd::Finished, ChildEnd::OutOfBudget),
                    unapplied: outcome.unapplied_steers,
                })
            }
            Err(CoreError::DeadlineExceeded { .. }) => Ok(ChildTurn {
                reply: String::new(),
                session_id,
                turns: tally.iterations,
                input_tokens: tally.input,
                output_tokens: tally.output,
                end: ChildEnd::OutOfBudget("wall time".into()),
                unapplied: steering.map(|s| s.take_unapplied()).unwrap_or_default(),
            }),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// Polls the turn to its end while forwarding the host's [`TurnControl`]:
/// each steer to `OpenHuman`'s steering queue, and a cancellation to the
/// turn's cancellation handle. The send future keeps being polled while the
/// cancellation is acknowledged, as `TurnCancellation::cancel` requires.
async fn drive<F, T>(
    send: F,
    control: &TurnControl,
    steering: &TurnSteering,
    cancellation: TurnCancellation,
) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::pin!(send);
    let mut listening = true;
    loop {
        tokio::select! {
            outcome = &mut send => return outcome,
            signal = control.next(), if listening => match signal {
                ControlSignal::Steer(text) => {
                    // A turn that has already ended refuses the steer and the
                    // send future is about to return: the text goes back on
                    // the host's queue, where `TurnControl::take_steers`
                    // finds it.
                    if let Err(refused) = steering.steer(text) {
                        control.steer(refused.text());
                        listening = false;
                    }
                }
                ControlSignal::Cancel => {
                    listening = false;
                    let cancellation = cancellation.clone();
                    tokio::spawn(async move { cancellation.cancel().await });
                }
            },
        }
    }
}

/// What a turn has spent so far, as the stop hook last saw it.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Tally {
    pub(super) iterations: u32,
    pub(super) input: u64,
    pub(super) output: u64,
}

/// Stops a turn before its next model call once a limit is reached.
///
/// A turn is *cut off* only if the child still had tool calls to run after the
/// hook fired; a final reply that happens to land on the limit finished.
pub(super) struct LimitHook {
    limits: Limits,
    belt: Arc<Belt>,
    state: Mutex<HookState>,
}

#[derive(Default)]
struct HookState {
    tally: Tally,
    tripped: Option<(String, u32)>,
}

impl LimitHook {
    pub(super) fn new(limits: Limits, belt: Arc<Belt>) -> Self {
        Self {
            limits,
            belt,
            state: Mutex::new(HookState::default()),
        }
    }

    pub(super) fn tally(&self) -> Tally {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tally
    }

    /// The limit that cut the turn short, if the child was still working.
    pub(super) fn cut_off(&self) -> Option<String> {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let (reason, calls_then) = state.tripped.as_ref()?;
        (self.belt.tool_calls() > *calls_then).then(|| reason.clone())
    }
}

#[async_trait::async_trait]
impl StopHook<TurnCost> for LimitHook {
    fn name(&self) -> &'static str {
        "supermath-child-limits"
    }

    async fn check(&self, state: &TurnState<'_>) -> StopDecision {
        let spent = state.cost.input_tokens + state.cost.output_tokens;
        let reason = if state.iteration >= self.limits.max_iterations {
            Some("iterations")
        } else if spent >= self.limits.max_tokens {
            Some("tokens")
        } else {
            None
        };
        let mut held = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        held.tally = Tally {
            iterations: state.iteration,
            input: state.cost.input_tokens,
            output: state.cost.output_tokens,
        };
        match reason {
            Some(reason) => {
                held.tripped
                    .get_or_insert_with(|| (reason.to_owned(), self.belt.tool_calls()));
                StopDecision::Stop {
                    reason: format!("{reason} budget reached"),
                }
            }
            None => StopDecision::Continue,
        }
    }
}
