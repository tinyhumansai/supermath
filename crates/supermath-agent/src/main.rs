//! `supermath-agent`: boot the runner, check the stack, or run the agent.
//!
//! ```text
//! supermath-agent check [project]
//! supermath-agent solve <project> <prompt-file> [turns]
//! ```
//!
//! `check` boots `OpenHuman` with Surplus Intelligence and Jev, registers the
//! agent for `project` (default `scratch`) with the composed toolkit and the
//! `agent_*` tools, lists them, and checks that `CortexDB` is ready.
//!
//! `solve` registers the same agent and runs it on the problem in
//! `prompt-file` for at most `turns` turns (default 1) in one session. Tool
//! calls are reported on stderr as they happen; each turn's reply goes to
//! stdout. Exit status: `0` on success, `1` otherwise.

use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::sync::Arc;

use openhuman_embed::Tool;
use supermath_agent::{Config, Runner};

const USAGE: &str = "\
usage: supermath-agent check [project]
       supermath-agent solve <project> <prompt-file> [turns]";

/// What the command line asked for.
enum Command {
    Check(String),
    Solve {
        project: String,
        prompt_file: String,
        turns: NonZeroUsize,
    },
}

/// The project `check` registers when none is named.
const DEFAULT_PROJECT: &str = "scratch";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["check"] => Command::Check(DEFAULT_PROJECT.to_string()),
        ["check", project] => Command::Check(project.to_string()),
        ["solve", project, prompt_file] => Command::Solve {
            project: project.to_string(),
            prompt_file: prompt_file.to_string(),
            turns: NonZeroUsize::MIN,
        },
        ["solve", project, prompt_file, turns] => match turns.parse() {
            Ok(turns) => Command::Solve {
                project: project.to_string(),
                prompt_file: prompt_file.to_string(),
                turns,
            },
            Err(_) => {
                eprintln!("error: turns must be a positive number, not `{turns}`\n{USAGE}");
                return ExitCode::from(1);
            }
        },
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
    let ran = match command {
        Command::Check(project) => runtime.block_on(check(&project)),
        Command::Solve {
            project,
            prompt_file,
            turns,
        } => runtime.block_on(solve(&project, &prompt_file, turns)),
    };
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

async fn check(project: &str) -> supermath_agent::Result<()> {
    let config = Config::from_env()?;
    // OpenHuman's logging: a daily file under `<workspace>/logs/`, filtered by
    // `RUST_LOG`.
    openhuman_embed::process::init_for_embedded(&config.workspace, false);

    let runner = Runner::start(&config).await?;
    let solver = runner.agent(project)?;
    println!(
        "runner: surplus {} model `{}` at {} effort; tool_search ranked by Jev",
        config.surplus_url, config.model, config.reasoning_effort
    );
    let tools = solver.host().adapt();
    let list = |agentic: bool| -> String {
        tools
            .iter()
            .filter(|tool| tool.kind().is_agentic() == agentic)
            .map(|tool| format!("{} ({:?})", tool.name(), tool.exposure()).to_lowercase())
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("agent: {} with {} tools", solver.agent().id(), tools.len());
    println!("tools: {}", list(false));
    println!("agent tools: {}", list(true));
    let servers: Vec<&str> = [
        ("jacobian", &config.jacobian_cmd),
        ("lean_lsp", &config.lean_lsp_cmd),
    ]
    .into_iter()
    .filter_map(|(name, command)| command.as_ref().map(|_| name))
    .collect();
    if !servers.is_empty() {
        println!("mcp servers: {}", servers.join(", "));
    }
    let memory = solver.host().toolkit().memory();
    memory.ready().await?;
    let scope = supermath_ledger::MemoryProject::new(solver.host().toolkit().project().slug())?;
    println!(
        "memory: {} ready, scope {}",
        config.toolkit.ledger.cortex_url,
        memory.scope(&scope)
    );
    Ok(())
}

async fn solve(project: &str, prompt_file: &str, turns: NonZeroUsize) -> supermath_agent::Result<()> {
    let prompt = std::fs::read_to_string(prompt_file).map_err(|error| {
        supermath_agent::Error::Prompt {
            path: prompt_file.to_owned(),
            reason: error.to_string(),
        }
    })?;
    let config = Config::from_env()?;
    openhuman_embed::process::init_for_embedded(&config.workspace, false);
    let runner = Runner::start(&config).await?;
    let solver = runner.agent(project)?;
    eprintln!(
        "solve: {} on `{}` at {} effort, up to {turns} turns",
        solver.agent().id(),
        config.model,
        config.reasoning_effort
    );
    let transcript = supermath_agent::solve(
        &solver,
        &prompt,
        turns,
        Arc::new(|line: String| eprintln!("{line}")),
    )
    .await?;
    for (number, reply) in transcript.replies.iter().enumerate() {
        println!("## turn {}\n\n{reply}\n", number + 1);
    }
    eprintln!(
        "solve: session {} ended after {} turns{}",
        transcript.session_id,
        transcript.replies.len(),
        if transcript.done() { ", done" } else { "" }
    );
    Ok(())
}
