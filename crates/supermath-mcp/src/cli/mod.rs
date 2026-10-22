//! The `supermath-mcp` command line.
//!
//! ```text
//! supermath-mcp serve [project]   # MCP over stdio
//! supermath-mcp check [project]   # Lean, Python, CortexDB, Gemini, arXiv
//! ```
//!
//! The project defaults to `scratch`. It comes from the command line only;
//! no tool argument can change it. [`parse`] turns arguments into a
//! [`Command`]; [`run`] executes one and returns the process exit code.

use std::process::ExitCode;
use std::sync::Arc;

use crate::check;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::server;
use crate::toolkit::Toolkit;

/// The usage line, printed for `--help` and after a usage error.
pub const USAGE: &str = "usage: supermath-mcp (serve | check) [project]";

/// The project used when none is named.
pub const DEFAULT_PROJECT: &str = "scratch";

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Serve MCP over stdio for `project`.
    Serve {
        /// The project name, before slugging.
        project: String,
    },
    /// Probe the backends, composing the toolkit for `project`.
    Check {
        /// The project name, before slugging.
        project: String,
    },
    /// Print the usage line.
    Help,
}

/// Parses the arguments after the program name.
///
/// # Errors
///
/// [`Error::Usage`] carrying the usage line when the arguments are not
/// `serve`, `check`, or `--help`, with at most one project.
pub fn parse(args: &[String]) -> Result<Command> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words[..] {
        ["serve"] => Ok(Command::Serve {
            project: DEFAULT_PROJECT.to_owned(),
        }),
        ["serve", project] => Ok(Command::Serve {
            project: project.to_owned(),
        }),
        ["check"] => Ok(Command::Check {
            project: DEFAULT_PROJECT.to_owned(),
        }),
        ["check", project] => Ok(Command::Check {
            project: project.to_owned(),
        }),
        ["-h" | "--help"] => Ok(Command::Help),
        _ => Err(Error::Usage(USAGE.to_owned())),
    }
}

/// Runs `command` against the environment. `Serve` returns when the client
/// disconnects; `Check` prints its report to standard output.
///
/// # Errors
///
/// Whatever configuring, composing, or serving fails with.
pub async fn execute(command: Command) -> Result<ExitCode> {
    match command {
        Command::Help => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Serve { project } => {
            let config = Config::from_env()?;
            let toolkit = Arc::new(Toolkit::compose(&config, &project)?);
            server::serve(toolkit).await?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { project } => {
            let config = Config::from_env()?;
            let toolkit = Toolkit::compose(&config, &project)?;
            let report = check::run(&toolkit, &config).await;
            print!("{report}");
            Ok(if report.passed() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
    }
}

/// Parses `args` (after the program name) and runs them, reporting any
/// error on standard error. Standard output is the MCP channel for `serve`,
/// so errors never go there.
pub async fn run(args: Vec<String>) -> ExitCode {
    let outcome = match parse(&args) {
        Ok(command) => execute(command).await,
        Err(error) => Err(error),
    };
    outcome.unwrap_or_else(|error| {
        eprintln!("error: {error}");
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod test;
