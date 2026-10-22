//! [`ScriptRunner`]: a child runner that plays a closure against the belt.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::belt::Belt;
use crate::child::{ChildEnd, ChildRequest, ChildRunner, ChildTurn};
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
