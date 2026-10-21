//! `supermath-agent`: solve one problem file and print the verdict.
//!
//! ```text
//! supermath-agent solve <problem.md>
//! ```
//!
//! Exit status: `0` proved or refuted, `2` not established, `1` on error.

use std::path::PathBuf;
use std::process::ExitCode;

use supermath_agent::{Config, Harness, Problem, Verdict};

const USAGE: &str = "usage: supermath-agent solve <problem.md>";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let path = match (args.next().as_deref(), args.next(), args.next()) {
        (Some("solve"), Some(path), None) => PathBuf::from(path),
        (Some("-h" | "--help"), None, None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(1);
        }
    };
    // Agent turns are deep async state machines; OpenHuman's runtime builder
    // sizes worker stacks for them.
    let runtime = match openhuman_embed::process::tokio_runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("error: cannot start the async runtime: {error}");
            return ExitCode::from(1);
        }
    };
    match runtime.block_on(solve(path)) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

async fn solve(path: PathBuf) -> supermath_agent::Result<ExitCode> {
    let config = Config::from_env()?;
    let problem = Problem::from_file(&path)?;
    eprintln!("solving `{}` with {}", problem.slug(), config.model);
    let harness = Harness::start(&config).await?;
    let report = harness.solve(&problem).await?;

    if let Some(error) = &report.recall_error {
        eprintln!("warning: recall failed, the run started without memory: {error}");
    }
    println!("{}", report.reply.trim());
    println!();
    println!("verdict: {}", report.verdict);
    match (&report.memory_event, &report.memory_error) {
        (Some(event), _) => eprintln!("stored in memory as {event}"),
        (None, Some(error)) => eprintln!("warning: the run was NOT stored in memory: {error}"),
        (None, None) => {}
    }
    Ok(match report.verdict {
        Verdict::Proved | Verdict::Refuted => ExitCode::SUCCESS,
        Verdict::NotEstablished(_) => ExitCode::from(2),
    })
}
