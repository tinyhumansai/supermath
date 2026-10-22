//! The formalization benchmark against a real stack, in process. Skipped
//! unless `SUPERMATH_LIVE_SUBAGENTS=1`.
//!
//! `live_subagents_formalize_bench` boots an `OpenHuman` runtime on the model
//! ladder, composes the real toolkit (`supermath-mcp`) for a bench project per
//! rung, builds the `agent_*` tools over its registry, runs the thirty
//! statements of `crates/supermath-subagents/bench/formalize/` through
//! `agent_formalize` once per rung, and prints the `checked` rate per rung.
//! It makes real model calls and real Lean checks, so it is never part of
//! `cargo test` by default.
//!
//! | Variable | Meaning |
//! | --- | --- |
//! | `SUPERMATH_LIVE_SUBAGENTS` | `1` to run |
//! | `LADDER_URL`, `LADDER_API_KEY` | the OpenAI-compatible ladder |
//! | `LEAN_URL`, `SUPERMATH_PROJECTS` | the Lean service and the directory it mounts |
//! | `SUPERMATH_BENCH_RUNGS` | comma-separated rungs; default `formalize` |
//! | `SUPERMATH_BENCH_LIMIT` | run only the first N cases |
//!
//! Every other toolkit variable (`PYTHON_URL`, `CORTEX_*`, ...) is read
//! as the agent reads it; an unset one leaves its tools answering
//! `BACKEND_UNAVAILABLE`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use openhuman_embed::{Provider, Runtime, RuntimeConfig, Workspace};
use supermath_mcp::{Config, Toolkit};
use supermath_subagents::bench::{formalize_cases, run_formalize};
use supermath_subagents::{Models, Subagents, SubagentsConfig};
use supermath_tool::{Budget, CallContext};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

#[test]
fn live_subagents_formalize_bench() {
    if env("SUPERMATH_LIVE_SUBAGENTS").as_deref() != Some("1") {
        eprintln!("skipped: set SUPERMATH_LIVE_SUBAGENTS=1 (see tests/live.rs for the rest)");
        return;
    }
    // Agent turns are deep async state machines; OpenHuman's runtime builder
    // sizes worker stacks for them.
    let runtime = openhuman_embed::process::tokio_runtime().unwrap();
    runtime.block_on(async { tokio::spawn(bench()).await.unwrap() });
}

async fn bench() {
    let ladder_url = env("LADDER_URL").expect("LADDER_URL");
    let ladder_key = env("LADDER_API_KEY").expect("LADDER_API_KEY");
    let rungs = env("SUPERMATH_BENCH_RUNGS").unwrap_or_else(|| "formalize".into());
    let mut cases = formalize_cases().unwrap();
    if let Some(limit) = env("SUPERMATH_BENCH_LIMIT").and_then(|v| v.parse().ok()) {
        cases.truncate(limit);
    }
    let config = Config::from_env().unwrap();

    let scratch = tempfile::tempdir().unwrap();
    let mut runtime_config = RuntimeConfig::default();
    runtime_config.local_ai.runtime_enabled = false;
    // The build future is large; boxing it keeps it off the caller's stack.
    let host = Box::pin(
        Runtime::builder()
            .config(runtime_config)
            .workspace(Workspace::Dir(scratch.path().to_path_buf()))
            .build(),
    )
    .await
    .unwrap();

    for rung in rungs.split(',').map(str::trim).filter(|r| !r.is_empty()) {
        let slug: String = rung
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let toolkit = Toolkit::compose(&config, &format!("bench-{slug}")).unwrap();
        let built = Subagents::new(
            &host,
            &toolkit.registry(),
            toolkit.call_log(),
            toolkit.project(),
            &SubagentsConfig {
                models: Models {
                    formalizer: rung.to_owned(),
                    ..Models::default()
                },
                provider: Some(Provider::openai_compatible(&ladder_url, &ladder_key)),
                progress: None,
            },
        )
        .unwrap();
        let formalize = built
            .tools()
            .iter()
            .find(|t| t.name() == "agent_formalize")
            .unwrap_or_else(|| panic!("agent_formalize unavailable: {:?}", built.unavailable()));
        let ctx = CallContext::new(toolkit.project().clone(), format!("bench-{slug}")).with_budget(
            Budget {
                wall_time_ms: 3_600_000,
                ..Budget::agentic(64, 1_000_000)
            },
        );
        let report = run_formalize(formalize.as_ref(), &ctx, rung, &cases).await;
        println!("{}", report.summary());
        for result in report.results.iter().filter(|r| !r.checked()) {
            println!(
                "  not checked: {} ({:?}, {:?})",
                result.id, result.outcome, result.verdict
            );
        }
    }
}
