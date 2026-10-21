//! The `supermath-agent` binary end to end, against loopback stubs of the
//! ladder, `CortexDB`, and the `OpenHuman` backend. Nothing reaches the
//! network.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

use serde_json::{Value, json};

use support::{Seen, Stub, completion, tool_call};

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

/// The provider stub: call `recall_memory` first, then answer with a proof.
fn provider() -> support::Handler {
    Arc::new(|seen: &Seen| {
        if !seen.path.ends_with("/chat/completions") {
            return (404, json!({ "error": "not stubbed" }));
        }
        let answered_tool = seen.body["messages"]
            .as_array()
            .is_some_and(|messages| messages.iter().any(|m| m["role"] == "tool"));
        if answered_tool {
            (
                200,
                completion("Induction on n: 1 = 1^2, and n^2 + (2n+1) = (n+1)^2.\nVERDICT: proved"),
            )
        } else {
            (
                200,
                tool_call("recall_memory", &json!({ "query": "odd sums" })),
            )
        }
    })
}

/// The `CortexDB` stub: one recalled note, and every write indexed.
fn cortex() -> support::Handler {
    Arc::new(|seen: &Seen| {
        if seen.path.starts_with("/v1/recall") {
            (
                200,
                json!({ "layers": { "events": [{ "content": { "text": "checked n <= 5" } }] } }),
            )
        } else if seen.path.starts_with("/v1/experience") {
            (
                200,
                json!({ "event_id": "evt-stub", "stages_completed": ["captured", "indexed"] }),
            )
        } else {
            (404, json!({ "error_code": "NOT_FOUND" }))
        }
    })
}

/// The `OpenHuman` backend stub: accepts anything.
fn backend() -> support::Handler {
    Arc::new(|_: &Seen| (200, json!({ "success": true, "data": {} })))
}

#[tokio::test(flavor = "multi_thread")]
async fn solves_a_problem_through_the_stack_and_stores_the_verdict() {
    let (ladder, memory, backend) = (
        Stub::start(provider()).await,
        Stub::start(cortex()).await,
        Stub::start(backend()).await,
    );
    let home = tempfile::tempdir().unwrap();
    let problem = home.path().join("odd-sums.md");
    std::fs::write(&problem, "Show 1+3+...+(2n-1) = n^2.").unwrap();
    let env = vec![
        ("LADDER_URL".into(), format!("{}/v1", ladder.url)),
        ("LADDER_API_KEY".into(), "ladder-key".into()),
        ("OPENROUTER_API_KEY".into(), "sk-or-stub".into()),
        ("CORTEX_API_URL".into(), memory.url.clone()),
        ("CORTEX_API_KEY".into(), "cortex-key".into()),
        ("SUPERMATH_BACKEND_URL".into(), backend.url.clone()),
        (
            "SUPERMATH_WORKSPACE".into(),
            home.path().join("workspace").display().to_string(),
        ),
    ];

    let output = run(&["solve", problem.to_str().unwrap()], env, home.path()).await;
    let (stdout, stderr) = (text(&output.stdout), text(&output.stderr));

    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(stdout.contains("verdict: proved"), "{stdout}");
    assert!(stderr.contains("tool calls: recall_memory"), "{stderr}");
    assert!(stderr.contains("stored in memory as evt-stub"), "{stderr}");

    // Memory: recalled before the turn and by the tool, written once after,
    // always at this problem's scope.
    let memory_seen = memory.seen();
    let recalls: Vec<&Seen> = memory_seen
        .iter()
        .filter(|seen| seen.path == "/v1/recall")
        .collect();
    assert_eq!(recalls.len(), 2, "{memory_seen:?}");
    assert_eq!(recalls[1].body["query"], "odd sums");
    let writes: Vec<&Seen> = memory_seen
        .iter()
        .filter(|seen| seen.path.starts_with("/v1/experience"))
        .collect();
    assert_eq!(writes.len(), 1);
    assert_eq!(
        writes[0].body["scope"],
        "ws:supermath/project:odd-sums/app:brain"
    );
    assert_eq!(writes[0].body["context"]["labels"][0], "verdict:proved");

    // The ladder served the turn on the default `flash` ladder at medium
    // reasoning effort, and the recalled note reached the model.
    // (`GET /v1/models` probes from OpenHuman's model catalog are skipped.)
    let turns: Vec<Seen> = ladder
        .seen()
        .into_iter()
        .filter(|seen| seen.path.ends_with("/chat/completions"))
        .collect();
    assert_eq!(turns.len(), 2, "one call for the tool, one for the answer");
    for turn in &turns {
        assert_eq!(turn.body["model"], "flash");
        assert_eq!(turn.body["reasoning_effort"], "medium", "{}", turn.body);
    }
    let first: String = turns[0].body["messages"].to_string();
    assert!(first.contains("checked n <= 5"), "{first}");
}

#[tokio::test]
async fn exits_two_when_the_reply_carries_no_verdict() {
    let ladder = Stub::start(Arc::new(|seen: &Seen| {
        if seen.path.ends_with("/chat/completions") {
            (200, completion("I am not sure."))
        } else {
            (404, Value::Null)
        }
    }))
    .await;
    // CortexDB is down: the run still answers, and says memory was not updated.
    let home = tempfile::tempdir().unwrap();
    let problem = home.path().join("hard.md");
    std::fs::write(&problem, "Is every even number > 2 a sum of two primes?").unwrap();
    let env = vec![
        ("LADDER_URL".into(), format!("{}/v1", ladder.url)),
        ("LADDER_API_KEY".into(), "k".into()),
        ("OPENROUTER_API_KEY".into(), "k".into()),
        ("CORTEX_API_URL".into(), "http://127.0.0.1:9".into()),
        ("CORTEX_API_KEY".into(), "k".into()),
        ("SUPERMATH_BACKEND_URL".into(), ladder.url.clone()),
        (
            "SUPERMATH_WORKSPACE".into(),
            home.path().join("workspace").display().to_string(),
        ),
    ];
    let output = run(&["solve", problem.to_str().unwrap()], env, home.path()).await;
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(text(&output.stdout).contains("no verdict line"));
    assert!(stderr.contains("recall failed"), "{stderr}");
    assert!(stderr.contains("NOT stored in memory"), "{stderr}");
    assert!(stderr.contains("tool calls: none"), "{stderr}");
}

#[tokio::test]
async fn reports_usage_and_configuration_errors() {
    let home = tempfile::tempdir().unwrap();

    let help = run(&["--help"], Vec::new(), home.path()).await;
    assert!(help.status.success());
    assert!(text(&help.stdout).contains("usage: supermath-agent solve"));

    let bad = run(&["prove", "x"], Vec::new(), home.path()).await;
    assert_eq!(bad.status.code(), Some(1));
    assert!(text(&bad.stderr).contains("usage:"));

    let unconfigured = run(&["solve", "p.md"], Vec::new(), home.path()).await;
    assert_eq!(unconfigured.status.code(), Some(1));
    assert!(
        text(&unconfigured.stderr).contains("`LADDER_API_KEY` is not set"),
        "{}",
        text(&unconfigured.stderr)
    );
}
