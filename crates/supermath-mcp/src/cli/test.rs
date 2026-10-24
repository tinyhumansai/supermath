//! Argument parsing, configuration, and the commands that need no backend.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::ExitCode;

use super::{
    Command, DEFAULT_PROJECT, Invocation, Options, USAGE, configure, execute, parse, run,
};
use crate::error::Error;

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

fn command(words: &[&str]) -> Command {
    parse(&args(words)).unwrap().command
}

#[test]
fn every_command_takes_an_optional_project() {
    let default = || DEFAULT_PROJECT.to_owned();
    assert_eq!(command(&["serve"]), Command::Serve { project: default() });
    assert_eq!(
        command(&["serve", "twin-primes"]),
        Command::Serve {
            project: "twin-primes".into()
        }
    );
    assert_eq!(command(&["check"]), Command::Check { project: default() });
    assert_eq!(
        command(&["check", "p"]),
        Command::Check {
            project: "p".into()
        }
    );
    assert_eq!(
        command(&["call", "calc_factor"]),
        Command::Call {
            tool: "calc_factor".into(),
            project: default()
        }
    );
    assert_eq!(
        command(&["call", "calc_factor", "p"]),
        Command::Call {
            tool: "calc_factor".into(),
            project: "p".into()
        }
    );
    assert_eq!(
        command(&["hive-tools"]),
        Command::HiveTools { project: default() }
    );
    assert_eq!(
        command(&["hive-tools", "p"]),
        Command::HiveTools {
            project: "p".into()
        }
    );
}

#[test]
fn options_are_read_wherever_they_appear() {
    let parsed = parse(&args(&[
        "--env-file",
        "a.env",
        "call",
        "lean_check",
        "--workspace",
        ".",
        "p",
    ]))
    .unwrap();
    assert_eq!(
        parsed,
        Invocation {
            command: Command::Call {
                tool: "lean_check".into(),
                project: "p".into()
            },
            options: Options {
                workspace: Some(PathBuf::from(".")),
                env_file: Some(PathBuf::from("a.env")),
            },
        }
    );
    assert_eq!(parse(&args(&["serve"])).unwrap().options, Options::default());
}

#[test]
fn help_flags_ask_for_the_usage_line() {
    assert_eq!(command(&["--help"]), Command::Help);
    assert_eq!(command(&["-h"]), Command::Help);
}

#[test]
fn anything_else_is_a_usage_error() {
    for bad in [
        &[][..],
        &["bogus"],
        &["serve", "a", "b"],
        &["check", "a", "b"],
        &["call"],
        &["call", "t", "a", "b"],
        &["hive-tools", "a", "b"],
        &["serve", "--workspace"],
        &["serve", "--env-file", "a", "--env-file", "b"],
    ] {
        let error = parse(&args(bad)).unwrap_err();
        assert_eq!(error.to_string(), USAGE, "{bad:?}");
    }
}

#[test]
fn configure_reads_the_env_file_and_makes_the_workspace_absolute() {
    let dir = tempfile::tempdir().unwrap();
    let env_file = dir.path().join("supermath.env");
    // A name no real environment sets, so the file's value is the one read.
    std::fs::write(&env_file, "CORTEX_SCOPE_ROOT=ws:from-file\n").unwrap();
    let config = configure(&Options {
        workspace: Some(PathBuf::from("candidate")),
        env_file: Some(env_file),
    })
    .unwrap();
    assert_eq!(config.ledger.scope_root, "ws:from-file");
    let workspace = config.files.workspace.unwrap();
    assert!(workspace.is_absolute(), "{workspace:?}");
    assert!(workspace.ends_with("candidate"));
}

#[test]
fn configure_fails_on_a_missing_env_file() {
    let dir = tempfile::tempdir().unwrap();
    let error = configure(&Options {
        workspace: None,
        env_file: Some(dir.path().join("absent.env")),
    })
    .unwrap_err();
    assert!(matches!(error, Error::EnvFile { .. }), "{error:?}");
}

#[tokio::test]
async fn help_succeeds_and_a_usage_error_fails() {
    let help = Invocation {
        command: Command::Help,
        options: Options::default(),
    };
    assert_eq!(execute(help).await.unwrap(), ExitCode::SUCCESS);
    assert_eq!(run(args(&["--help"])).await, ExitCode::SUCCESS);
    assert_eq!(run(args(&["bogus"])).await, ExitCode::FAILURE);
}

#[tokio::test]
async fn hive_tools_prints_and_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let invocation = Invocation {
        command: Command::HiveTools {
            project: "demo".into(),
        },
        options: Options {
            workspace: Some(dir.path().to_path_buf()),
            env_file: None,
        },
    };
    assert_eq!(execute(invocation).await.unwrap(), ExitCode::SUCCESS);
}
