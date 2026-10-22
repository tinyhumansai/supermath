//! `supermath-agent`: boot the runner and report whether the stack is ready.
//!
//! ```text
//! supermath-agent check [project]
//! ```
//!
//! Boots `OpenHuman` with Surplus Intelligence and Jev, registers the agent for
//! `project` (default `scratch`) with the composed toolkit and the `agent_*`
//! tools, lists them, and checks that `CortexDB` is ready. Exit status: `0`
//! ready, `1` otherwise.

use std::process::ExitCode;

use openhuman_embed::Tool;
use supermath_agent::{Config, Runner};

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
