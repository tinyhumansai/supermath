//! The belt: the primitives one child turn may call, and a record of what it
//! called.
//!
//! A [`Belt`] holds a [`ToolRegistry`] scoped to the child's listed
//! primitives and dispatches every child call through it, so each call is
//! logged in the shared [`CallLog`] under its own call id. It also journals
//! what the registry answered. The `agent_*` tools read that journal, never
//! the child's reply, to learn what was written and recorded.
//!
//! [`HostTool`] adapts one registry tool into `OpenHuman`'s `Tool` trait, so
//! the child sees each primitive as a real tool with its own schema.
//!
//! A host that has to react while the child works, rather than after its
//! turn, attaches a [`Watcher`] with [`Belt::with_watcher`] and sees each
//! journaled call as it lands.

mod host;
mod types;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;
use supermath_tool::{CallContext, CallLog, Envelope, ToolRegistry};

pub use host::HostTool;
pub(crate) use types::Watcher;
pub use types::{CallNote, Progress};

/// Most journal entries kept per turn; the oldest are dropped past it.
const MAX_NOTES: usize = 2000;

/// The primitives of one child turn and the journal of its calls.
pub struct Belt {
    parent: &'static str,
    registry: ToolRegistry,
    log: Arc<dyn CallLog>,
    base: CallContext,
    notes: Mutex<Vec<CallNote>>,
    calls: AtomicU32,
    progress: Option<Arc<dyn Progress>>,
    watcher: Option<Arc<dyn Watcher>>,
}

impl std::fmt::Debug for Belt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Belt")
            .field("parent", &self.parent)
            .field("tools", &self.registry)
            .field("calls", &self.calls.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl Belt {
    /// A belt for the turn that `parent` (an `agent_*` tool) runs under
    /// `base`, reaching only the tools in `registry`.
    #[must_use]
    pub fn new(
        parent: &'static str,
        registry: ToolRegistry,
        log: Arc<dyn CallLog>,
        base: CallContext,
        progress: Option<Arc<dyn Progress>>,
    ) -> Self {
        Self {
            parent,
            registry,
            log,
            base,
            notes: Mutex::new(Vec::new()),
            calls: AtomicU32::new(0),
            progress,
            watcher: None,
        }
    }

    /// This belt with `watcher` told of every call once it is journaled.
    #[must_use]
    pub(crate) fn with_watcher(mut self, watcher: Arc<dyn Watcher>) -> Self {
        self.watcher = Some(watcher);
        self
    }

    /// The tools the child may call, in name order.
    #[must_use]
    pub fn tool_names(&self) -> Vec<&'static str> {
        self.registry.names().collect()
    }

    /// Calls `tool` as the child, logs it, and journals the answer.
    ///
    /// A name outside the belt answers `NOT_FOUND`; the child cannot reach a
    /// tool its scope does not list.
    pub async fn call(&self, tool: &str, args: Value) -> Envelope {
        let number = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let mut ctx = self.base.clone();
        ctx.call_id = format!("{}.{tool}.{number}", self.base.call_id);
        if let Some(progress) = &self.progress {
            progress.tool_started(self.parent, tool);
        }
        let envelope = self
            .registry
            .dispatch(self.log.as_ref(), tool, &ctx, args.clone())
            .await;
        if let Some(progress) = &self.progress {
            progress.tool_ended(self.parent, tool, envelope.outcome);
        }
        let note = CallNote {
            tool: tool.to_owned(),
            call_id: ctx.call_id,
            args,
            envelope: envelope.clone(),
        };
        if let Some(watcher) = &self.watcher {
            watcher.observed(&note);
        }
        let mut notes = self.notes.lock().unwrap_or_else(PoisonError::into_inner);
        if notes.len() >= MAX_NOTES {
            notes.remove(0);
        }
        notes.push(note);
        envelope
    }

    /// How many calls the child has made.
    #[must_use]
    pub fn tool_calls(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }

    /// Every call the child made, oldest first.
    #[must_use]
    pub fn notes(&self) -> Vec<CallNote> {
        self.notes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The calls to `tool` that answered `ok`, oldest first.
    #[must_use]
    pub fn succeeded(&self, tool: &str) -> Vec<CallNote> {
        self.notes()
            .into_iter()
            .filter(|note| {
                note.tool == tool && note.envelope.outcome == supermath_tool::Outcome::Ok
            })
            .collect()
    }

    /// The belt as `OpenHuman` host tools, one per registry tool.
    #[must_use]
    pub fn host_tools(self: &Arc<Self>) -> Vec<Box<dyn openhuman_embed::Tool>> {
        self.registry
            .iter()
            .map(|tool| {
                Box::new(HostTool::new(Arc::clone(self), Arc::clone(tool)))
                    as Box<dyn openhuman_embed::Tool>
            })
            .collect()
    }
}

#[cfg(test)]
mod test;
