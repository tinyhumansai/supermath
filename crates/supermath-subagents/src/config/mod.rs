//! Which ladder rung each child runs on, read from the environment.
//!
//! | Variable | Default | Used by |
//! | --- | --- | --- |
//! | `SUPERMATH_READER_MODEL` | `flash` | `agent_digest_paper` |
//! | `SUPERMATH_FORMALIZER_MODEL` | `formalize` | `agent_formalize`, `agent_refute` |
//! | `SUPERMATH_REVIEWER_MODEL` | `reasoning` | `agent_review_alignment` |
//!
//! The values are ladder names, so a change of model is a change to the
//! ladder's config and these variables, never to this crate.

use crate::error::{Error, Result};

/// The model each child runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Models {
    /// The paper reader's rung (`SUPERMATH_READER_MODEL`).
    pub reader: String,
    /// The formalizer's and the refuter's rung (`SUPERMATH_FORMALIZER_MODEL`).
    pub formalizer: String,
    /// The alignment reviewer's rung (`SUPERMATH_REVIEWER_MODEL`).
    pub reviewer: String,
}

impl Default for Models {
    fn default() -> Self {
        Self {
            reader: "flash".into(),
            formalizer: "formalize".into(),
            reviewer: "reasoning".into(),
        }
    }
}

impl Models {
    /// Reads the three variables from the process environment.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] for a variable that is set but blank or holds
    /// whitespace.
    pub fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the three variables through `lookup`; an unset one keeps its
    /// default.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] for a variable that is set but blank or holds
    /// whitespace.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let defaults = Self::default();
        let read = |name: &'static str, default: String| match lookup(name) {
            None => Ok(default),
            Some(value) if !value.is_empty() && !value.contains(char::is_whitespace) => Ok(value),
            Some(value) => Err(Error::InvalidConfig { name, value }),
        };
        Ok(Self {
            reader: read("SUPERMATH_READER_MODEL", defaults.reader)?,
            formalizer: read("SUPERMATH_FORMALIZER_MODEL", defaults.formalizer)?,
            reviewer: read("SUPERMATH_REVIEWER_MODEL", defaults.reviewer)?,
        })
    }
}

#[cfg(test)]
mod test;
