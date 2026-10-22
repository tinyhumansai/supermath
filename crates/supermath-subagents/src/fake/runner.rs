//! [`ScriptRunner`]: a child runner that plays a closure against the belt.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::belt::Belt;
use crate::child::{ChildEnd, ChildRequest, ChildRunner, ChildTurn, ControlSignal, TurnControl};
use crate::limits::Limits;

/// What a script does with the prompt and belt of one turn.
pub(crate) type Script = Box<
    dyn Fn(String, Arc<Belt>) -> Pin<Box<dyn Future<Output = Result<ChildTurn, String>> + Send>>
        + Send
        + Sync,
>;

/// Boxes an async closure as a [`Script`].
pub(crate) fn script<F, Fut>(f: F) -> Script
where
    F: Fn(String, Arc<Belt>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<ChildTurn, String>> + Send + 'static,
{
    Box::new(move |prompt, belt| Box::pin(f(prompt, belt)))
}

/// A finished turn replying `reply`.
pub(crate) fn turn(reply: &str) -> ChildTurn {
    ChildTurn {
        reply: reply.into(),
        session_id: "sess-1".into(),
        turns: 3,
        input_tokens: 100,
        output_tokens: 20,
        end: ChildEnd::Finished,
        unapplied: Vec::new(),
    }
}

/// Plays a script in place of an `OpenHuman` agent, recording what it was
/// asked.
pub(crate) struct ScriptRunner {
    script: Script,
    scopes: Mutex<Vec<Vec<&'static str>>>,
    limits: Mutex<Vec<Limits>>,
}

impl std::fmt::Debug for ScriptRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScriptRunner")
    }
}

impl ScriptRunner {
    pub(crate) fn new(script: Script) -> Arc<Self> {
        Arc::new(Self {
            script,
            scopes: Mutex::new(Vec::new()),
            limits: Mutex::new(Vec::new()),
        })
    }

    /// The tool names each turn's belt offered.
    pub(crate) fn scopes(&self) -> Vec<Vec<&'static str>> {
        self.scopes.lock().unwrap().clone()
    }

    /// The limits each turn ran under.
    pub(crate) fn limits(&self) -> Vec<Limits> {
        self.limits.lock().unwrap().clone()
    }
}

#[async_trait]
impl ChildRunner for ScriptRunner {
    fn model(&self) -> &'static str {
        "scripted"
    }

    async fn run(&self, request: ChildRequest) -> Result<ChildTurn, String> {
        self.scopes.lock().unwrap().push(request.belt.tool_names());
        self.limits.lock().unwrap().push(request.limits);
        (self.script)(request.prompt, request.belt).await
    }
}

/// What a controlled script does with one turn's prompt, belt, and the
/// host's control handle.
pub(crate) type Controlled = Box<
    dyn Fn(
            String,
            Arc<Belt>,
            TurnControl,
        ) -> Pin<Box<dyn Future<Output = Result<ChildTurn, String>> + Send>>
        + Send
        + Sync,
>;

/// Boxes an async closure as a [`Controlled`] script.
pub(crate) fn controlled<F, Fut>(f: F) -> Controlled
where
    F: Fn(String, Arc<Belt>, TurnControl) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<ChildTurn, String>> + Send + 'static,
{
    Box::new(move |prompt, belt, control| Box::pin(f(prompt, belt, control)))
}

/// A turn the host cancelled.
pub(crate) fn cancelled() -> ChildTurn {
    ChildTurn {
        end: ChildEnd::OutOfBudget("cancelled".into()),
        ..turn("")
    }
}

/// Waits until the host cancels the turn, ignoring steers, as a side that
/// keeps working until it is stopped would.
pub(crate) async fn until_cancelled(control: &TurnControl) -> Result<ChildTurn, String> {
    while control.next().await != ControlSignal::Cancel {}
    Ok(cancelled())
}

/// Plays a controlled script for a runner the host steers, recording each
/// turn's prompt and limits.
pub(crate) struct ControlRunner {
    script: Controlled,
    prompts: Mutex<Vec<String>>,
    limits: Mutex<Vec<Limits>>,
}

impl std::fmt::Debug for ControlRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ControlRunner")
    }
}

impl ControlRunner {
    pub(crate) fn new(script: Controlled) -> Arc<Self> {
        Arc::new(Self {
            script,
            prompts: Mutex::new(Vec::new()),
            limits: Mutex::new(Vec::new()),
        })
    }

    /// The prompt of each turn, in order.
    pub(crate) fn prompts(&self) -> Vec<String> {
        self.prompts.lock().unwrap().clone()
    }

    /// The limits of each turn, in order.
    pub(crate) fn limits(&self) -> Vec<Limits> {
        self.limits.lock().unwrap().clone()
    }
}

#[async_trait]
impl ChildRunner for ControlRunner {
    fn model(&self) -> &'static str {
        "controlled"
    }

    async fn run(&self, request: ChildRequest) -> Result<ChildTurn, String> {
        self.prompts.lock().unwrap().push(request.prompt.clone());
        self.limits.lock().unwrap().push(request.limits);
        let control = request
            .control
            .expect("a controlled runner is driven with a control handle");
        (self.script)(request.prompt, request.belt, control).await
    }
}
