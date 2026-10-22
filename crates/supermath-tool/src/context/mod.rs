//! [`CallContext`]: what a host passes to every tool call.

mod types;

pub use types::{Cancellation, EvidenceOrigin, Project};

use crate::Budget;

/// Everything a tool learns about a call besides its arguments. The host
/// builds it; a tool cannot widen it.
#[derive(Debug, Clone)]
pub struct CallContext {
    /// The project the call acts on.
    pub project: Project,
    /// Identifies the call in the [`CallLog`](crate::CallLog).
    pub call_id: String,
    /// Where evidence this call produces comes from.
    pub evidence_origin: EvidenceOrigin,
    /// What the call may spend.
    pub budget: Budget,
    /// Fires when the caller gives up.
    pub cancel: Cancellation,
}

impl CallContext {
    /// A context for `project` with the given `call_id`, collected evidence,
    /// the default budget, and a fresh cancellation signal.
    #[must_use]
    pub fn new(project: Project, call_id: impl Into<String>) -> Self {
        Self {
            project,
            call_id: call_id.into(),
            evidence_origin: EvidenceOrigin::Collected,
            budget: Budget::default(),
            cancel: Cancellation::new(),
        }
    }

    /// Replaces the evidence origin.
    #[must_use]
    pub fn with_evidence_origin(mut self, origin: EvidenceOrigin) -> Self {
        self.evidence_origin = origin;
        self
    }

    /// Replaces the budget.
    #[must_use]
    pub fn with_budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }
}

#[cfg(test)]
mod test;
