//! Configuration lookup.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use super::{DEFAULT_PROJECTS, FilesConfig, PROJECTS_ENV};

#[test]
fn defaults_when_unset_or_blank() {
    let default = FilesConfig::default();
    assert_eq!(default.projects, PathBuf::from(DEFAULT_PROJECTS));
    assert_eq!(FilesConfig::from_lookup(|_| None), default);
    assert_eq!(FilesConfig::from_lookup(|_| Some("  ".into())), default);
}

#[test]
fn reads_the_projects_root_from_the_variable() {
    let config = FilesConfig::from_lookup(|name| {
        (name == PROJECTS_ENV).then(|| "/data/projects".to_owned())
    });
    assert_eq!(config.projects, PathBuf::from("/data/projects"));
}

#[test]
fn from_env_reads_the_process_environment() {
    // Whatever the environment holds, the result is a usable root.
    assert!(!FilesConfig::from_env().projects.as_os_str().is_empty());
}
