//! Configuration: where the projects live.

use std::path::PathBuf;

use crate::workspace::Workspace;

/// The environment variable that roots the projects directory.
pub const PROJECTS_ENV: &str = "SUPERMATH_PROJECTS";

/// The environment variable naming one project directory to work in instead
/// of a projects root; see [`Workspace::single`].
pub const WORKSPACE_ENV: &str = "SUPERMATH_WORKSPACE";

/// The projects root when [`PROJECTS_ENV`] is unset.
pub const DEFAULT_PROJECTS: &str = "./.supermath/projects";

/// Settings for the files crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesConfig {
    /// The directory holding `<slug>/lean` and `<slug>/python` for every
    /// project.
    pub projects: PathBuf,
    /// One project directory to use in place of `projects`, when set.
    pub workspace: Option<PathBuf>,
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            projects: PathBuf::from(DEFAULT_PROJECTS),
            workspace: None,
        }
    }
}

impl FilesConfig {
    /// Reads `SUPERMATH_PROJECTS` and `SUPERMATH_WORKSPACE` from the process
    /// environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// As [`FilesConfig::from_env`], reading variables through `lookup`. An
    /// empty value counts as unset.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let read = |name| lookup(name).filter(|value| !value.trim().is_empty());
        Self {
            projects: read(PROJECTS_ENV).map_or_else(|| PathBuf::from(DEFAULT_PROJECTS), From::from),
            workspace: read(WORKSPACE_ENV).map(PathBuf::from),
        }
    }

    /// The workspace these settings describe: the single project directory
    /// when one is set, the projects root otherwise.
    #[must_use]
    pub fn open(&self) -> Workspace {
        match &self.workspace {
            Some(dir) => Workspace::single(dir),
            None => Workspace::new(&self.projects),
        }
    }
}

#[cfg(test)]
mod test;
