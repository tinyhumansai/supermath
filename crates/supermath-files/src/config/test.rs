//! Configuration lookup.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use super::{DEFAULT_PROJECTS, FilesConfig, PROJECTS_ENV, WORKSPACE_ENV};

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

#[test]
fn reads_a_single_workspace_from_its_variable() {
    let config = FilesConfig::from_lookup(|name| {
        (name == WORKSPACE_ENV).then(|| "/work/candidate".to_owned())
    });
    assert_eq!(config.workspace, Some(PathBuf::from("/work/candidate")));
    assert_eq!(config.projects, PathBuf::from(DEFAULT_PROJECTS));
    let workspace = config.open();
    assert_eq!(
        workspace.project_dir("demo").unwrap(),
        PathBuf::from("/work/candidate")
    );
}

#[test]
fn opens_the_projects_root_without_a_single_workspace() {
    let workspace = FilesConfig::default().open();
    assert_eq!(
        workspace.project_dir("demo").unwrap(),
        PathBuf::from(DEFAULT_PROJECTS).join("demo")
    );
}
