//! The `supermath-mcp` command line.
//!
//! ```text
//! supermath-mcp serve [project]          # MCP over stdio
//! supermath-mcp check [project]          # Lean, Python, CortexDB, Gemini, arXiv
//! supermath-mcp call <tool> [project]    # one call: JSON args on stdin, envelope on stdout
//! supermath-mcp hive-tools [project]     # the tools as tinyhivemind executable tools
//!
//! options, after the command:
//!   --workspace <dir>    use <dir> as the project's directory (see `Workspace::single`)
//!   --env-file <file>    read unset variables from a `KEY=VALUE` file
//! ```
//!
//! The project defaults to `scratch`. It comes from the command line only;
//! no tool argument can change it. `serve` and `call` keep the call log on
//! disk ([`Toolkit::compose_persistent`]), so a `ledger_record` can cite a
//! call another process made. [`parse`] turns arguments into an
//! [`Invocation`]; [`run`] executes one and returns the process exit code.

mod call;
mod env_file;
mod hive;

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use crate::check;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::server;
use crate::toolkit::Toolkit;

pub use call::call;
pub use env_file::EnvFile;
pub use hive::{AGENTIC_TIMEOUT_MS, PRIMITIVE_TIMEOUT_MS, Program, declarations};

/// The usage text, printed for `--help` and after a usage error.
pub const USAGE: &str = "usage: supermath-mcp (serve [project] | check [project] | \
call <tool> [project] | hive-tools [project]) [--workspace <dir>] [--env-file <file>]";

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
    /// Call one tool with arguments from standard input.
    Call {
        /// The tool's name.
        tool: String,
        /// The project name, before slugging.
        project: String,
    },
    /// Print every tool as a tinyhivemind executable-tool declaration.
    HiveTools {
        /// The project name, before slugging.
        project: String,
    },
    /// Print the usage line.
    Help,
}

/// Options that apply to every command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// `--workspace`: the project's directory, in place of the projects root.
    pub workspace: Option<PathBuf>,
    /// `--env-file`: where unset variables are read from.
    pub env_file: Option<PathBuf>,
}

/// A parsed command line: the command and its options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// What to do.
    pub command: Command,
    /// How to configure it.
    pub options: Options,
}

/// Parses the arguments after the program name.
///
/// # Errors
///
/// [`Error::Usage`] carrying the usage text when the arguments are not one
/// of the commands with at most one project, or an option lacks its value
/// or is repeated.
pub fn parse(args: &[String]) -> Result<Invocation> {
    let usage = || Error::Usage(USAGE.to_owned());
    let mut options = Options::default();
    let mut words = Vec::new();
    let mut rest = args.iter();
    while let Some(word) = rest.next() {
        let slot = match word.as_str() {
            "--workspace" => &mut options.workspace,
            "--env-file" => &mut options.env_file,
            _ => {
                words.push(word.as_str());
                continue;
            }
        };
        let value = rest.next().ok_or_else(usage)?;
        if slot.replace(PathBuf::from(value)).is_some() {
            return Err(usage());
        }
    }
    let project = |name: Option<&&str>| name.map_or(DEFAULT_PROJECT, |p| p).to_owned();
    let command = match words[..] {
        ["serve", ref rest @ ..] if rest.len() <= 1 => Command::Serve {
            project: project(rest.first()),
        },
        ["check", ref rest @ ..] if rest.len() <= 1 => Command::Check {
            project: project(rest.first()),
        },
        ["call", tool, ref rest @ ..] if rest.len() <= 1 => Command::Call {
            tool: tool.to_owned(),
            project: project(rest.first()),
        },
        ["hive-tools", ref rest @ ..] if rest.len() <= 1 => Command::HiveTools {
            project: project(rest.first()),
        },
        ["-h" | "--help"] => Command::Help,
        _ => return Err(usage()),
    };
    Ok(Invocation { command, options })
}

/// Reads the configuration `options` describe: the process environment,
/// then the `--env-file` for anything it leaves unset, then `--workspace`.
///
/// # Errors
///
/// [`Error::EnvFile`] if the env file cannot be read; [`Error::Usage`] if
/// the workspace cannot be made absolute; as [`Config::from_lookup`].
pub fn configure(options: &Options) -> Result<Config> {
    let file = match &options.env_file {
        Some(path) => EnvFile::read(path)?,
        None => EnvFile::default(),
    };
    let mut config = Config::from_lookup(|name| {
        std::env::var(name)
            .ok()
            .or_else(|| file.get(name).map(str::to_owned))
    })?;
    if let Some(dir) = &options.workspace {
        // Absolute, so a host that hands the path to a service elsewhere
        // never sees `.`.
        let dir = std::path::absolute(dir).map_err(|_| Error::Usage(USAGE.to_owned()))?;
        config.files.workspace = Some(dir);
    }
    Ok(config)
}

/// Runs `invocation` against the environment. `Serve` returns when the
/// client disconnects; the others print to standard output.
///
/// # Errors
///
/// Whatever configuring, composing, or serving fails with.
pub async fn execute(invocation: Invocation) -> Result<ExitCode> {
    let Invocation { command, options } = invocation;
    match command {
        Command::Help => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Serve { project } => {
            let config = configure(&options)?;
            let toolkit = Arc::new(Toolkit::compose_persistent(&config, &project)?);
            server::serve(toolkit).await?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Check { project } => {
            let config = configure(&options)?;
            let toolkit = Toolkit::compose(&config, &project)?;
            let report = check::run(&toolkit, &config).await;
            print!("{report}");
            Ok(if report.passed() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Command::Call { tool, project } => {
            let config = configure(&options)?;
            let toolkit = Toolkit::compose_persistent(&config, &project)?;
            let mut input = String::new();
            // Unreadable input is treated as none; the tool's schema then
            // decides whether `{}` is enough.
            let _ = std::io::stdin().read_to_string(&mut input);
            println!("{}", call(&toolkit, &tool, &input).await);
            Ok(ExitCode::SUCCESS)
        }
        Command::HiveTools { project } => {
            let config = configure(&options)?;
            let toolkit = Toolkit::compose(&config, &project)?;
            let program = Program {
                binary: std::env::current_exe()
                    .map_or_else(|_| "supermath-mcp".to_owned(), |p| p.display().to_string()),
                project: toolkit.project().slug().to_owned(),
                env_file: options
                    .env_file
                    .as_deref()
                    .and_then(|path| std::path::absolute(path).ok())
                    .map(|path| path.display().to_string()),
            };
            let tools = declarations(&toolkit.registry(), &program);
            println!("{tools:#}");
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Parses `args` (after the program name) and runs them, reporting any
/// error on standard error. Standard output is the MCP channel for `serve`
/// and the answer for `call`, so errors never go there.
pub async fn run(args: Vec<String>) -> ExitCode {
    let outcome = match parse(&args) {
        Ok(invocation) => execute(invocation).await,
        Err(error) => Err(error),
    };
    outcome.unwrap_or_else(|error| {
        eprintln!("error: {error}");
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod test;
