//! `supermath-agent`: boot the runner and report whether the stack is ready.
//!
//! ```text
//! supermath-agent check [project]
//! ```
//!
//! Boots `OpenHuman` with the ladder and Jev, registers the agent for
//! `project` (default `scratch`) with the toolkit, and checks that `CortexDB`
//! is ready. Exit status: `0` ready, `1` otherwise.

use std::process::ExitCode;

use supermath_agent::{Config, Project, Runner, Toolkit};

const USAGE: &str = "usage: supermath-agent check [project]";

/// The project `check` registers when none is named.
const DEFAULT_PROJECT: &str = "scratch";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let project = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["check"] => DEFAULT_PROJECT.to_string(),
        ["check", project] => project.to_string(),
        ["-h" | "--help"] => {
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
    match runtime.block_on(check(&project)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

async fn check(project: &str) -> supermath_agent::Result<()> {
    let config = Config::from_env()?;
    let project = Project::new(project)?;
    // OpenHuman's logging: a daily file under `<workspace>/logs/`, filtered by
    // `RUST_LOG`.
    openhuman_embed::process::init_for_embedded(&config.workspace, false);

    let runner = Runner::start(&config).await?;
    let agent = runner.agent(&project)?;
    println!(
        "runner: ladder {} model `{}` at {} effort; tool_search ranked by Jev",
        config.ladder_url, config.model, config.reasoning_effort
    );
    let tools: Vec<String> = Toolkit::new(runner.memory().clone(), project.clone())
        .tools()
        .iter()
        .map(|tool| format!("{} ({:?})", tool.name(), tool.exposure()).to_lowercase())
        .collect();
    println!("agent: {} with {}", agent.id(), tools.join(", "));
    runner.memory().ready().await?;
    println!(
        "memory: {} ready, scope {}",
        config.cortex_url,
        runner.memory().scope(&project)
    );
    Ok(())
}
