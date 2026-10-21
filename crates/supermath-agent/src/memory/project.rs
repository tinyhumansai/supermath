//! The project a memory scope belongs to.
//!
//! A project name becomes part of a `CortexDB` scope, so it is normalised to
//! a slug once, here, and only ever comes from the host, never from model
//! output or a tool argument.

use crate::error::{Error, Result};

/// A normalised project name: lowercase letters, digits, and single dashes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Project(String);

impl Project {
    /// Normalises `name` with [`slugify`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidProject`] when `name` has no usable characters.
    pub fn new(name: &str) -> Result<Self> {
        slugify(name)
            .map(Self)
            .ok_or_else(|| Error::InvalidProject(name.to_string()))
    }

    /// The slug.
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
