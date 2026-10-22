//! The budget of one child turn: iterations, tokens, and wall time.
//!
//! Every `agent_*` call may carry a `budget` argument that lowers or raises
//! its tool's defaults up to [`Limits::MAX`]; the result is admitted against
//! the caller's [`Budget`](supermath_tool::Budget) before the child runs.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::Demand;

/// What one child turn may spend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Model calls the child may make.
    pub max_iterations: u32,
    /// Input plus output tokens the child may spend.
    pub max_tokens: u64,
    /// Wall-clock seconds the turn may take.
    pub wall_secs: u64,
}

/// The `budget` argument every `agent_*` tool accepts. Any field omitted keeps
/// the tool's default.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BudgetArg {
    /// Replaces [`Limits::max_iterations`].
    pub max_iterations: Option<u32>,
    /// Replaces [`Limits::max_tokens`].
    pub max_tokens: Option<u64>,
    /// Replaces [`Limits::wall_secs`].
    pub wall_secs: Option<u64>,
}

impl Limits {
    /// The ceiling no call may exceed: 64 iterations, 1,000,000 tokens, 1,800 s.
    pub const MAX: Self = Self::new(64, 1_000_000, 1_800);
    /// The default of the reader, the formalizer, and the refuter: 24
    /// iterations, 200,000 tokens, 600 s.
    pub const DEFAULT: Self = Self::new(24, 200_000, 600);
    /// The alignment reviewer's default: 12 iterations, 100,000 tokens, 300 s.
    pub const REVIEW: Self = Self::new(12, 100_000, 300);

    /// Limits with the given fields.
    #[must_use]
    pub const fn new(max_iterations: u32, max_tokens: u64, wall_secs: u64) -> Self {
        Self {
            max_iterations,
            max_tokens,
            wall_secs,
        }
    }

    /// These limits with `iterations` in place of [`Limits::max_iterations`].
    #[must_use]
    pub const fn with_iterations(mut self, iterations: u32) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// Applies a caller's `budget` argument.
    ///
    /// # Errors
    ///
    /// A message for a zero field or a field above [`Limits::MAX`]; the tool
    /// answers it as `INVALID_INPUT`.
    pub fn apply(mut self, arg: BudgetArg) -> Result<Self, String> {
        if let Some(value) = arg.max_iterations {
            self.max_iterations = Self::bounded("max_iterations", value, Self::MAX.max_iterations)?;
        }
        if let Some(value) = arg.max_tokens {
            self.max_tokens = Self::bounded("max_tokens", value, Self::MAX.max_tokens)?;
        }
        if let Some(value) = arg.wall_secs {
            self.wall_secs = Self::bounded("wall_secs", value, Self::MAX.wall_secs)?;
        }
        Ok(self)
    }

    fn bounded<T: PartialOrd + Copy + Default + std::fmt::Display>(
        name: &str,
        value: T,
        max: T,
    ) -> Result<T, String> {
        if value == T::default() {
            Err(format!("budget.{name} must be at least 1"))
        } else if value > max {
            Err(format!("budget.{name} is {value}, above the maximum {max}"))
        } else {
            Ok(value)
        }
    }

    /// What admitting these limits asks of the caller's budget.
    #[must_use]
    pub fn demand(&self) -> Demand {
        Demand {
            wall_time_ms: self.wall_secs.saturating_mul(1000),
            iterations: self.max_iterations,
            tokens: self.max_tokens,
            ..Demand::default()
        }
    }

    /// The wall-clock allowance as a duration.
    #[must_use]
    pub fn wall(&self) -> Duration {
        Duration::from_secs(self.wall_secs)
    }
}

/// The JSON schema of the `budget` argument, shared by the four tools.
#[must_use]
pub fn budget_schema() -> Value {
    json!({
        "type": "object",
        "description": "Lowers or raises this call's budget, up to 64 iterations, 1,000,000 tokens, and 1,800 s.",
        "properties": {
            "max_iterations": {"type": "integer", "minimum": 1, "maximum": Limits::MAX.max_iterations},
            "max_tokens": {"type": "integer", "minimum": 1, "maximum": Limits::MAX.max_tokens},
            "wall_secs": {"type": "integer", "minimum": 1, "maximum": Limits::MAX.wall_secs}
        },
        "additionalProperties": false
    })
}

#[cfg(test)]
mod test;
