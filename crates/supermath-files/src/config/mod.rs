//! Configuration: where the projects live.

use std::path::PathBuf;

/// The environment variable that roots the projects directory.
pub const PROJECTS_ENV: &str = "SUPERMATH_PROJECTS";

/// The projects root when [`PROJECTS_ENV`] is unset.
pub const DEFAULT_PROJECTS: &str = "./.supermath/projects";

/// Settings for the files crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesConfig {
    /// The directory holding `<slug>/lean` and `<slug>/python` for every
    /// project.
    pub projects: PathBuf,
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            projects: PathBuf::from(DEFAULT_PROJECTS),
        }
    }
}

impl FilesConfig {
    /// Reads `SUPERMATH_PROJECTS` from the process environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// As [`FilesConfig::from_env`], reading variables through `lookup`. An
    /// empty value counts as unset.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        lookup(PROJECTS_ENV)
            .filter(|value| !value.trim().is_empty())
            .map_or_else(Self::default, |value| Self {
                projects: PathBuf::from(value),
            })
    }
}

#[cfg(test)]
mod test;
