//! A problem to solve: a slug that names its memory, and the statement.
//!
//! The slug is derived from the file name and nothing else, because it
//! becomes part of a `CortexDB` scope. Deriving it from the statement or from
//! model output would let text decide which memory a run reads.

use std::path::Path;

use crate::error::{Error, Result};

/// One problem statement and the slug its memory is filed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    slug: String,
    statement: String,
}

impl Problem {
    /// Builds a problem from a slug source and a statement.
    ///
    /// `name` is normalised with [`slugify`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnnamedProblem`] when `name` has no usable
    /// characters, and [`Error::EmptyProblem`] when the statement is blank.
    pub fn new(name: &str, statement: impl Into<String>) -> Result<Self> {
        let slug = slugify(name).ok_or_else(|| Error::UnnamedProblem(name.to_string()))?;
        let statement = statement.into().trim().to_string();
        if statement.is_empty() {
            return Err(Error::EmptyProblem(slug));
        }
        Ok(Self { slug, statement })
    }

    /// Reads a problem from a Markdown file; the slug is the file stem.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProblemFile`] when the file cannot be read, and the
    /// errors of [`Problem::new`] otherwise.
    pub fn from_file(path: &Path) -> Result<Self> {
        let statement = std::fs::read_to_string(path).map_err(|source| Error::ProblemFile {
            path: path.display().to_string(),
            source,
        })?;
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        Self::new(stem, statement)
    }

    /// The slug that names this problem's memory.
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The problem statement, trimmed.
    #[must_use]
    pub fn statement(&self) -> &str {
        &self.statement
    }
}

/// Lowercases `name` and replaces every character outside `[a-z0-9-]` with
/// `-`, collapsing runs and trimming dashes from both ends.
///
/// Returns `None` when nothing usable is left.
#[must_use]
pub fn slugify(name: &str) -> Option<String> {
    let mut slug = String::with_capacity(name.len());
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    (!slug.is_empty()).then(|| slug.to_string())
}

#[cfg(test)]
mod test;
