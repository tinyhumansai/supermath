//! `supermath-agent check` end to end, against loopback stubs of Surplus,
//! `CortexDB`, and the `OpenHuman` backend. Nothing reaches the network.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

use serde_json::{Value, json};

use support::{Seen, Stub};

const BINARY: &str = env!("CARGO_BIN_EXE_supermath-agent");

/// Runs the binary with only the given environment, plus `PATH`.
async fn run(args: &[&str], env: Vec<(String, String)>, home: &Path) -> Output {
    let args: Vec<String> = args.iter().map(ToString::to_string).collect();
    let home = home.to_path_buf();
    tokio::task::spawn_blocking(move || {
        Command::new(BINARY)
            .args(&args)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            // `cargo llvm-cov` profiles the child through these; clearing
            // them would drop the binary from the coverage report.
            .envs(std::env::vars().filter(|(name, _)| {
                name.starts_with("LLVM_") || name.starts_with("CARGO_LLVM_COV")
            }))
            // OpenHuman keeps process-wide state under these; a test must
            // never touch the operator's real ones.
            .env("HOME", &home)
            .env("OPENHUMAN_WORKSPACE", home.join("openhuman"))
            .envs(env)
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// A stub that answers `/v1/admin/ready` with `report` and anything else
/// with an empty success.
async fn server(report: Value) -> Stub {
    Stub::start(Arc::new(move |seen: &Seen| {
        if seen.path == "/v1/admin/ready" {
            (200, report.clone())
        } else {
            (200, json!({ "success": true, "data": {} }))
        }
    }))
    .await
}

fn env(stub: &Stub, home: &Path) -> Vec<(String, String)> {
    vec![
        ("SURPLUS_URL".into(), format!("{}/v1", stub.url)),
        ("SURPLUS_API_KEY".into(), "surplus-key".into()),
        ("OPENROUTER_API_KEY".into(), "sk-or-stub".into()),
        ("CORTEX_API_URL".into(), stub.url.clone()),
        ("CORTEX_API_KEY".into(), "cortex-key".into()),
        ("SUPERMATH_BACKEND_URL".into(), stub.url.clone()),
        (
            "SUPERMATH_WORKSPACE".into(),
            home.join("workspace").display().to_string(),
        ),
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn check_boots_the_runner_and_reports_a_ready_stack() {
    let stub = server(json!({ "ready": true, "degraded": false })).await;
    let home = tempfile::tempdir().unwrap();
    let mut env = env(&stub, home.path());
    env.push((
        "SUPERMATH_JACOBIAN_CMD".into(),
        "jacobian-mcp --stdio".into(),
    ));
    env.push(("SUPERMATH_LEAN_LSP_CMD".into(), "uvx lean-lsp-mcp".into()));
    let output = run(&["check", "Twin Primes"], env, home.path()).await;
    let (stdout, stderr) = (text(&output.stdout), text(&output.stderr));

    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("model `deepseek-v4.1-flash` at medium effort; tool_search ranked by Jev"),
        "{stdout}"
    );
    assert!(
        stdout.contains("agent: supermath-twin-primes with 35 tools"),
        "{stdout}"
    );
    let line = |prefix: &str| {
        stdout
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap_or_else(|| panic!("no `{prefix}` line in {stdout}"))
            .to_string()
    };
    let tools = line("tools: ");
    for listed in [
        "recall_memory (direct)",
        "calc_factor (deferred)",
        "lean_check (",
        "ledger_record (",
    ] {
        assert!(tools.contains(listed), "{listed} in {tools}");
    }
    let agent_tools = line("agent tools: ");
    for name in [
        "agent_digest_paper",
        "agent_formalize",
        "agent_refute",
        "agent_review_alignment",
        "agent_settle",
    ] {
        assert!(agent_tools.contains(name), "{name} in {agent_tools}");
    }
    assert!(!stdout.contains("unavailable"), "{stdout}");
    assert!(
        stdout.contains("mcp servers: jacobian, lean_lsp"),
        "{stdout}"
    );
    assert!(
        stdout.contains("scope ws:supermath/project:twin-primes/app:brain"),
        "{stdout}"
    );
    // Booting runs no turn: nothing asked Surplus for a completion.
    assert!(
        !stub
            .seen()
            .iter()
            .any(|seen| seen.path.ends_with("/chat/completions"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn check_fails_when_memory_is_not_ready() {
    let stub =
        server(json!({ "ready": true, "degraded": true, "embedding_provider": "mock" })).await;
    let home = tempfile::tempdir().unwrap();
    let output = run(&["check"], env(&stub, home.path()), home.path()).await;
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("memory is not ready: degraded"), "{stderr}");
    assert!(!text(&output.stdout).contains("mcp servers"));
    assert!(text(&output.stdout).contains("agent: supermath-scratch"));
}

#[tokio::test]
async fn reports_usage_and_configuration_errors() {
    let home = tempfile::tempdir().unwrap();

    let help = run(&["--help"], Vec::new(), home.path()).await;
    assert!(help.status.success());
    assert!(text(&help.stdout).contains("usage: supermath-agent check"));

    let bad = run(&["solve", "x"], Vec::new(), home.path()).await;
    assert_eq!(bad.status.code(), Some(1));
    assert!(text(&bad.stderr).contains("usage:"));

    let unconfigured = run(&["check"], Vec::new(), home.path()).await;
    assert_eq!(unconfigured.status.code(), Some(1));
    assert!(
        text(&unconfigured.stderr).contains("`SURPLUS_API_KEY` is not set"),
        "{}",
        text(&unconfigured.stderr)
    );

    let stub = server(json!({ "ready": true })).await;
    let unnamed = run(&["check", "!!"], env(&stub, home.path()), home.path()).await;
    assert_eq!(unnamed.status.code(), Some(1));
    assert!(text(&unnamed.stderr).contains("invalid project name \"!!\""));
}
