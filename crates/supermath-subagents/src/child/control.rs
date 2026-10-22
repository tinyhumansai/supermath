//! [`TurnControl`]: the host's hold on one running child turn.
//!
//! A host that runs a child for a long time needs two things the turn itself
//! cannot provide: a way to add a message while the child works, and a way
//! to stop it. `OpenHuman` has both per turn (`TurnSteering`,
//! `TurnCancellation`), but a [`ChildRunner`](super::ChildRunner) is a port
//! that fakes implement too, so the host speaks to this crate-owned handle and
//! [`OpenHumanChild`](super::OpenHumanChild) forwards it.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::Notify;

/// What the runner should do next for the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlSignal {
    /// Add this text to the turn as a user message.
    Steer(String),
    /// Stop the turn.
    Cancel,
}

#[derive(Debug, Default)]
struct Inner {
    steers: Mutex<VecDeque<String>>,
    cancelled: AtomicBool,
    wake: Notify,
}

/// A handle that steers or cancels one child turn. Clones address the same
/// turn; the host keeps one and hands one to the runner in
/// [`ChildRequest::control`](super::ChildRequest::control).
#[derive(Debug, Clone, Default)]
pub struct TurnControl(Arc<Inner>);

impl TurnControl {
    /// A handle with nothing queued.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `text` for the turn. The runner applies it at the turn's next
    /// safe checkpoint, never in the middle of a tool call.
    pub fn steer(&self, text: impl Into<String>) {
        self.0
            .steers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(text.into());
        self.0.wake.notify_one();
    }

    /// Asks the runner to stop the turn. Calling it again is harmless.
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::SeqCst);
        self.0.wake.notify_one();
    }

    /// Whether [`cancel`](Self::cancel) was called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    /// Takes every steer not yet handed to the runner, oldest first.
    #[must_use]
    pub fn take_steers(&self) -> Vec<String> {
        self.0
            .steers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drain(..)
            .collect()
    }

    /// Waits for the next thing to do: a cancellation first, then queued
    /// steers in order. Once cancelled, every call answers
    /// [`ControlSignal::Cancel`], so a runner stops asking after the first.
    pub async fn next(&self) -> ControlSignal {
        loop {
            if self.is_cancelled() {
                return ControlSignal::Cancel;
            }
            let next = self
                .0
                .steers
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .pop_front();
            if let Some(text) = next {
                return ControlSignal::Steer(text);
            }
            self.0.wake.notified().await;
        }
    }
}
