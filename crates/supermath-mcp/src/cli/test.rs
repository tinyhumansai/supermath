//! Argument parsing and the commands that need no backend.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::ExitCode;

use super::{Command, DEFAULT_PROJECT, USAGE, execute, parse, run};

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

#[test]
fn serve_and_check_take_an_optional_project() {
    assert_eq!(
        parse(&args(&["serve"])).unwrap(),
        Command::Serve {
            project: DEFAULT_PROJECT.into()
        }
    );
    assert_eq!(
        parse(&args(&["serve", "twin-primes"])).unwrap(),
        Command::Serve {
            project: "twin-primes".into()
        }
    );
    assert_eq!(
        parse(&args(&["check"])).unwrap(),
        Command::Check {
            project: DEFAULT_PROJECT.into()
        }
    );
    assert_eq!(
        parse(&args(&["check", "p"])).unwrap(),
        Command::Check {
            project: "p".into()
        }
    );
}

#[test]
fn help_flags_ask_for_the_usage_line() {
    assert_eq!(parse(&args(&["--help"])).unwrap(), Command::Help);
    assert_eq!(parse(&args(&["-h"])).unwrap(), Command::Help);
}

#[test]
fn anything_else_is_a_usage_error() {
    for bad in [
        &[][..],
        &["bogus"],
        &["serve", "a", "b"],
        &["check", "a", "b"],
    ] {
        let error = parse(&args(bad)).unwrap_err();
        assert_eq!(error.to_string(), USAGE);
    }
}

#[tokio::test]
async fn help_succeeds_and_a_usage_error_fails() {
    assert_eq!(execute(Command::Help).await.unwrap(), ExitCode::SUCCESS);
    assert_eq!(run(args(&["--help"])).await, ExitCode::SUCCESS);
    assert_eq!(run(args(&["bogus"])).await, ExitCode::FAILURE);
}
