//! Which Surplus model each child runs on, read from the environment.
//!
//! | Variable | Default | Used by |
//! | --- | --- | --- |
//! | `SUPERMATH_READER_MODEL` | `deepseek-v4.1-flash` | `agent_digest_paper` |
//! | `SUPERMATH_FORMALIZER_MODEL` | `deepseek-v4-pro` | `agent_formalize`, `agent_refute` |
//! | `SUPERMATH_REVIEWER_MODEL` | `glm-5.3` | `agent_review_alignment` |
//! | `SUPERMATH_DUEL_MODEL` | `deepseek-v4-pro` | both sides of `agent_settle` |
//!
//! The values are Surplus Intelligence model names, so a change of model is a
//! change to these variables, never to this crate.
//!
//! [`DuelSettings`] reads how `agent_settle` keeps its two sides on task:
//!
//! | Variable | Default | Meaning |
//! | --- | --- | --- |
//! | `SUPERMATH_DUEL_DRIFT_CALLS` | `12` | tool calls without progress before a nudge |
//! | `SUPERMATH_DUEL_MAX_ROUNDS` | `4` | turns per side, each starting from a host-written summary |
//! | `SUPERMATH_DUEL_DIGEST_EVERY` | `8` | a side's own tool calls between two partner digests |

use crate::error::{Error, Result};

/// The model each child runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Models {
    /// The paper reader's model (`SUPERMATH_READER_MODEL`).
    pub reader: String,
    /// The formalizer's and the refuter's model (`SUPERMATH_FORMALIZER_MODEL`).
    pub formalizer: String,
    /// The alignment reviewer's model (`SUPERMATH_REVIEWER_MODEL`).
    pub reviewer: String,
    /// The model both duel sides run on (`SUPERMATH_DUEL_MODEL`): one model,
    /// so neither side starts with a stronger reasoner.
    pub duel: String,
}

impl Default for Models {
    fn default() -> Self {
        Self {
            reader: "deepseek-v4.1-flash".into(),
            formalizer: "deepseek-v4-pro".into(),
            reviewer: "glm-5.3".into(),
            duel: "deepseek-v4-pro".into(),
        }
    }
}

impl Models {
    /// Reads the four variables from the process environment.
    ///
    /// # Errors
    ///
    /// As [`Models::from_lookup`].
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the four variables through `lookup`; an unset one keeps its
    /// default.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] for a variable that is set but blank or holds
    /// whitespace; [`Error::ReviewerIsFormalizer`] when the reviewer would
    /// run on the formalizer's model.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let defaults = Self::default();
        let read = |name: &'static str, default: String| match lookup(name) {
            None => Ok(default),
            Some(value) if !value.is_empty() && !value.contains(char::is_whitespace) => Ok(value),
            Some(value) => Err(Error::InvalidConfig { name, value }),
        };
        let models = Self {
            reader: read("SUPERMATH_READER_MODEL", defaults.reader)?,
            formalizer: read("SUPERMATH_FORMALIZER_MODEL", defaults.formalizer)?,
            reviewer: read("SUPERMATH_REVIEWER_MODEL", defaults.reviewer)?,
            duel: read("SUPERMATH_DUEL_MODEL", defaults.duel)?,
        };
        models.check()?;
        Ok(models)
    }

    /// Checks the rule between the models: the reviewer runs on a different
    /// model from the formalizer, so the model that wrote a statement is not
    /// the one that judges it.
    ///
    /// # Errors
    ///
    /// [`Error::ReviewerIsFormalizer`] when the two are the same model.
    pub fn check(&self) -> Result<()> {
        if self.reviewer == self.formalizer {
            Err(Error::ReviewerIsFormalizer(self.reviewer.clone()))
        } else {
            Ok(())
        }
    }
}

/// How `agent_settle` paces and polices its two sides.
///
/// There is deliberately no setting that moves budget from one side to the
/// other: both sides get equal limits for the whole duel, and a side that
/// stops early leaves its remainder unspent (ADR 0009). That rules out
/// favouring whichever side looks closer, which needs a measure of
/// closeness we do not trust yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelSettings {
    /// Tool calls a side may make without progress before the host nudges it
    /// (`SUPERMATH_DUEL_DRIFT_CALLS`); the third drift in a row ends its turn.
    pub drift_calls: u32,
    /// Turns per side (`SUPERMATH_DUEL_MAX_ROUNDS`). A turn that ends with
    /// budget left is followed by one that starts from a host-written
    /// summary, which clears a context full of Lean errors.
    pub max_rounds: u32,
    /// A side's own tool calls between two digests of partner posts
    /// (`SUPERMATH_DUEL_DIGEST_EVERY`), so messages never arrive faster than
    /// a side can act on them.
    pub digest_every: u32,
}

impl Default for DuelSettings {
    fn default() -> Self {
        Self {
            drift_calls: 12,
            max_rounds: 4,
            digest_every: 8,
        }
    }
}

impl DuelSettings {
    /// Reads the three variables from the process environment.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] for a variable that is set but is not a
    /// whole number of at least 1.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the three variables through `lookup`; an unset one keeps its
    /// default.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] for a variable that is set but is not a
    /// whole number of at least 1.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let defaults = Self::default();
        let read = |name: &'static str, default: u32| match lookup(name) {
            None => Ok(default),
            Some(value) => match value.trim().parse::<u32>() {
                Ok(n) if n >= 1 => Ok(n),
                _ => Err(Error::InvalidConfig { name, value }),
            },
        };
        Ok(Self {
            drift_calls: read("SUPERMATH_DUEL_DRIFT_CALLS", defaults.drift_calls)?,
            max_rounds: read("SUPERMATH_DUEL_MAX_ROUNDS", defaults.max_rounds)?,
            digest_every: read("SUPERMATH_DUEL_DIGEST_EVERY", defaults.digest_every)?,
        })
    }
}

#[cfg(test)]
mod test;
