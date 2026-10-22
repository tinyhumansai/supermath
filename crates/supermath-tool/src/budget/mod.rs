//! Budgets and admission.
//!
//! Every call is admitted against a [`Budget`] before it runs. A call that
//! asks for more than the budget allows is refused with `BUDGET_EXCEEDED`
//! without touching its backend.
//!
//! ```
//! use supermath_tool::{Budget, Demand, ErrorCode, Envelope};
//!
//! let budget = Budget::default();
//! assert!(budget.admit(&Demand { rows: 10, ..Demand::default() }).is_ok());
//! let refused = budget.admit(&Demand { rows: 10_000, ..Demand::default() }).unwrap_err();
//! assert_eq!(Envelope::from(refused).code(), Some(ErrorCode::BudgetExceeded));
//! ```

use serde::{Deserialize, Serialize};

use crate::{Envelope, ErrorCode};

/// What a call may spend. Agentic limits are `None` for primitives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// Wall time in milliseconds.
    pub wall_time_ms: u64,
    /// Rows a list may return.
    pub rows: usize,
    /// Bytes of input or output.
    pub bytes: u64,
    /// Model iterations (agentic tools only).
    pub iterations: Option<u32>,
    /// Model tokens (agentic tools only).
    pub tokens: Option<u64>,
}

impl Default for Budget {
    /// Thirty seconds, fifty rows, one mebibyte, no agentic limits.
    fn default() -> Self {
        Self {
            wall_time_ms: 30_000,
            rows: crate::MAX_LIMIT,
            bytes: 1 << 20,
            iterations: None,
            tokens: None,
        }
    }
}

/// What a call asks to spend. Zero means "nothing of this kind".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Demand {
    /// Wall time the call expects to need, in milliseconds.
    pub wall_time_ms: u64,
    /// Rows the call will return.
    pub rows: usize,
    /// Bytes the call will read or write.
    pub bytes: u64,
    /// Model iterations the call will take.
    pub iterations: u32,
    /// Model tokens the call will spend.
    pub tokens: u64,
}

/// Why admission refused a call. Converts to a `BUDGET_EXCEEDED` envelope.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{resource} budget exceeded: asked for {requested}, limit is {limit}")]
pub struct Refusal {
    /// The exhausted resource: `wall_time_ms`, `rows`, `bytes`,
    /// `iterations`, or `tokens`.
    pub resource: &'static str,
    /// What the call asked for.
    pub requested: u64,
    /// What the budget allows.
    pub limit: u64,
}

impl From<Refusal> for Envelope {
    fn from(refusal: Refusal) -> Self {
        Envelope::error(ErrorCode::BudgetExceeded, refusal.to_string())
    }
}

impl Budget {
    /// A budget for an agentic tool: the default caps plus `iterations` and
    /// `tokens`.
    #[must_use]
    pub fn agentic(iterations: u32, tokens: u64) -> Self {
        Self {
            iterations: Some(iterations),
            tokens: Some(tokens),
            ..Self::default()
        }
    }

    /// Admits `demand` or refuses with the first exhausted resource.
    ///
    /// An agentic limit that is `None` is not enforced.
    ///
    /// # Errors
    ///
    /// Returns a [`Refusal`] naming the first resource the demand exceeds.
    pub fn admit(&self, demand: &Demand) -> Result<(), Refusal> {
        let checks = [
            ("wall_time_ms", demand.wall_time_ms, Some(self.wall_time_ms)),
            ("rows", demand.rows as u64, Some(self.rows as u64)),
            ("bytes", demand.bytes, Some(self.bytes)),
            (
                "iterations",
                u64::from(demand.iterations),
                self.iterations.map(u64::from),
            ),
            ("tokens", demand.tokens, self.tokens),
        ];
        for (resource, requested, limit) in checks {
            if let Some(limit) = limit
                && requested > limit
            {
                return Err(Refusal {
                    resource,
                    requested,
                    limit,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test;
