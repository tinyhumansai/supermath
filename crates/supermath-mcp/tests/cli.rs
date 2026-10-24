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

/// Runs `supermath-mcp call <tool> demo --project-dir <dir>` with `input` on
/// standard input.
fn call(dir: &std::path::Path, tool: &str, input: &str, extra: &[&str]) -> std::process::Output {
    use std::io::Write as _;
    let mut child = binary()
        .args(["call", tool, "demo", "--project-dir"])
        .arg(dir)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn answer(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn call_prints_the_envelope_and_logs_it_for_later_processes() {
    let dir = tempfile::tempdir().unwrap();
    let first = call(dir.path(), "calc_factor", r#"{"n": 360}"#, &[]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first = answer(&first);
    assert_eq!(first["outcome"], "ok", "{first}");

    let second = answer(&call(dir.path(), "calc_factor", r#"{"n": 7}"#, &[]));
    assert_ne!(
        first["call_id"], second["call_id"],
        "ids are unique across processes"
    );
    let log = std::fs::read_to_string(dir.path().join(".log/calls.jsonl")).unwrap();
    assert_eq!(log.lines().count(), 2, "{log}");
    assert!(log.contains(first["call_id"].as_str().unwrap()));
}

#[test]
fn call_answers_an_error_envelope_with_success() {
    let dir = tempfile::tempdir().unwrap();
    let out = call(dir.path(), "no_such_tool", "{}", &[]);
    assert!(out.status.success(), "an envelope is an answer");
    assert_eq!(answer(&out)["error"]["code"], "NOT_FOUND");
}

#[test]
fn call_writes_into_the_workspace_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let out = call(
        dir.path(),
        "file_write",
        r#"{"path": "lean/Supermath/Hello.lean", "content": "theorem t : True := trivial\n"}"#,
        &[],
    );
    assert_eq!(answer(&out)["outcome"], "ok", "{}", answer(&out));
    assert!(dir.path().join("lean/Supermath/Hello.lean").is_file());
}

#[test]
fn call_fails_without_an_answer_on_an_unreadable_env_file() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("absent.env");
    let out = call(
        dir.path(),
        "calc_factor",
        "{}",
        &["--env-file", missing.to_str().unwrap()],
    );
    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "no envelope without a configuration");
    assert!(String::from_utf8_lossy(&out.stderr).contains("env file"));
}

#[test]
fn hive_tools_declares_every_tool_against_this_binary() {
    let dir = tempfile::tempdir().unwrap();
    let env_file = dir.path().join("supermath.env");
    std::fs::write(&env_file, "").unwrap();
    let out = binary()
        .args(["hive-tools", "Demo Problem", "--env-file"])
        .arg(&env_file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tools: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(tools.len(), 30);
    let command = &tools[0]["command"];
    assert_eq!(command[0], env!("CARGO_BIN_EXE_supermath-mcp"));
    assert_eq!(command[3], "demo-problem");
    assert_eq!(command[7], env_file.to_str().unwrap());
}

const STATEMENT: &str = "import Mathlib\n\nnamespace Supermath\n\n\
                         def Conjecture : Prop := 2 + 2 = 4\n\nend Supermath\n";

#[test]
fn loop_new_writes_a_loop_folder_that_runs_this_binary() {
    let root = tempfile::tempdir().unwrap();
    let statement = root.path().join("Statement.lean");
    std::fs::write(&statement, STATEMENT).unwrap();
    let dir = root.path().join("loops/two-add-two");
    let out = binary()
        .args(["loop", "new", "Two Add Two"])
        .arg(&statement)
        .arg(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), dir.to_str().unwrap());
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("rsi.json")).unwrap()).unwrap();
    assert_eq!(manifest["name"], "two-add-two");
    let scorer = manifest["score"]["scorers"][0]["run"].as_str().unwrap();
    assert!(scorer.starts_with(env!("CARGO_BIN_EXE_supermath-mcp")), "{scorer}");
    assert!(dir.join("subject/lean/Supermath/Statement.lean").is_file());

    // A second run never overwrites it; a missing statement file fails too.
    let again = binary()
        .args(["loop", "new", "two-add-two"])
        .arg(&statement)
        .arg(&dir)
        .output()
        .unwrap();
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("already exists"));
    let missing = binary()
        .args(["loop", "new", "x", "/nonexistent/Statement.lean"])
        .output()
        .unwrap();
    assert!(!missing.status.success());
}

#[test]
fn score_proof_needs_the_rsi_ralph_variables() {
    let out = binary().args(["score", "proof", "demo"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("RSI_CANDIDATE is not set"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn score_proof_checks_a_copy_and_writes_the_result() {
    let lean = LeanService::start().await;
    let root = tempfile::tempdir().unwrap();
    let candidate = root.path().join("candidate");
    let put = |path: &str, text: &str| {
        let path = candidate.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    put("lean/Supermath/Statement.lean", STATEMENT);
    put(
        "lean/Supermath/Prove.lean",
        "import Supermath.Statement\n\nnamespace Supermath\n\ntheorem proof : Conjecture := by\n  norm_num\n\nend Supermath\n",
    );
    let out_dir = root.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = binary()
        .args(["score", "proof", "demo"])
        .env("RSI_CANDIDATE", &candidate)
        .env("RSI_OUT", &out_dir)
        .env("LEAN_URL", &lean.stub.base)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("METRIC proof="), "{stdout}");
    let result: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out_dir.join("result.json")).unwrap())
            .unwrap();
    let score = result["score"].as_f64().unwrap();
    assert!((0.0..=1.0).contains(&score), "{result}");
    assert_eq!(stdout.trim(), format!("METRIC proof={score}"));

    // Both checks ran on the copy, never on the candidate.
    let sent = lean.stub.bodies("/check");
    let copy = out_dir.join("project");
    let paths: Vec<&str> = sent.iter().map(|body| body["path"].as_str().unwrap()).collect();
    assert_eq!(
        paths,
        ["lean/Supermath/Prove.lean", "lean/Supermath/ScoreProved.lean"]
    );
    assert!(sent.iter().all(|body| body["root"] == copy.to_str().unwrap()));
    assert!(!candidate.join("lean/Supermath/ScoreProved.lean").exists());
}
