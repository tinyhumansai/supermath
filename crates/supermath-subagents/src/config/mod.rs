//! Which Surplus model each child runs on, read from the environment.
//!
//! | Variable | Default | Used by |
//! | --- | --- | --- |
//! | `SUPERMATH_READER_MODEL` | `deepseek-v4.1-flash` | `agent_digest_paper` |
//! | `SUPERMATH_FORMALIZER_MODEL` | `deepseek-v4-pro` | `agent_formalize`, `agent_refute` |
//! | `SUPERMATH_REVIEWER_MODEL` | `deepseek-v4-pro` | `agent_review_alignment` |
//!
//! The values are Surplus Intelligence model names, so a change of model is a
//! change to these variables, never to this crate.

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
}

impl Default for Models {
    fn default() -> Self {
        Self {
            reader: "deepseek-v4.1-flash".into(),
            formalizer: "deepseek-v4-pro".into(),
            reviewer: "deepseek-v4-pro".into(),
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
