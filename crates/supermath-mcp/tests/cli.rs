//! The binary's exit codes and output outside `serve`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::process::{Command, Stdio};

use support::{LeanService, Reply, Stub};

const EMPTY_FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:opensearch="http://a9.com/-/spec/opensearch/1.1/">
<opensearch:totalResults>0</opensearch:totalResults>
</feed>"#;

fn binary() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_supermath-mcp"));
    command.env_clear().stdin(Stdio::null());
    support::keep_coverage_env(&mut command);
    command
}

#[test]
fn help_prints_the_usage_line_and_succeeds() {
    let out = binary().arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("usage: supermath-mcp"));
}

#[test]
fn a_bad_command_fails_with_usage_on_stderr_and_nothing_on_stdout() {
    let out = binary().arg("bogus").output().unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "stdout is the MCP channel");
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage: supermath-mcp"));
}

#[test]
fn a_bad_configuration_fails_on_stderr() {
    let out = binary()
        .arg("check")
        .env("SUPERMATH_WEB_DAILY_CAP", "lots")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("SUPERMATH_WEB_DAILY_CAP"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn check_reports_each_backend_and_exits_zero_when_none_failed() {
    let lean = LeanService::start().await;
    let arxiv = Stub::start(|_| Reply::atom(EMPTY_FEED)).await;
    let out = binary()
        .args(["check", "demo"])
        .env("LEAN_URL", &lean.stub.base)
        .env("SUPERMATH_ARXIV_API_URL", &arxiv.base)
        .env("GEMINI_API_KEY", "g-secret")
        .output()
        .unwrap();
    let shown = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{shown}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        shown.contains("lean: toolchain leanprover/lean4:v4.20.0"),
        "{shown}"
    );
    assert!(shown.contains("skipped python"), "{shown}");
    assert!(!shown.contains("g-secret"), "{shown}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn check_exits_nonzero_when_a_configured_backend_is_down() {
    let arxiv = Stub::start(|_| Reply::status(500)).await;
    let out = binary()
        .arg("check")
        .env("LEAN_URL", "http://127.0.0.1:1")
        .env("SUPERMATH_ARXIV_API_URL", &arxiv.base)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("FAILED"));
}
