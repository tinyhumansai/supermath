//! The three tools through their public API, against the loopback stub that
//! replays answers recorded from the real service.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use supermath_files::{FileObserver, ProjectPath};
use supermath_python::args_sha256;
use supermath_tool::contract::{self, Fixture};
use supermath_tool::{Budget, CallContext, Envelope, ErrorCode, Outcome};
use support::{Recorded, Reply, Setup, Stub, ctx, ctx_with, fixture};

const SUNFLOWER: &str =
    "def largest(n, k, seed):\n    return {\"size\": n * k, \"seed\": seed, \"exact\": True}\n";
const EULER: &str = "def prime_value(n):\n    return True\n";

fn file_sha(dir: &Path, path: &str) -> String {
    let bytes = std::fs::read(dir.join("demo").join(path)).unwrap();
    hex::encode(Sha256::digest(bytes))
}

/// Routes by the entry or predicate named in the request.
fn routes(dir: PathBuf) -> impl Fn(&Recorded) -> Reply + Send + Sync + 'static {
    move |request| {
        let sha = request
            .body
            .get("path")
            .and_then(Value::as_str)
            .filter(|p| dir.join("demo").join(p).is_file())
            .map(|p| file_sha(&dir, p))
            .unwrap_or_default();
        let named = request.body["entry"]
            .as_str()
            .or_else(|| request.body["predicate"].as_str())
            .unwrap_or_default();
        let limit = |name: &str| {
            let mut body = fixture("run_timeout");
            body["limit"] = json!(name);
            Reply::ok(body)
        };
        let search_limit = |checked: Option<u64>| {
            let mut body = fixture("run_timeout");
            body["search"] = checked.map_or(Value::Null, |n| json!({"checked": n}));
            Reply::ok(body)
        };
        let reply = match (request.method.as_str(), request.path.as_str(), named) {
            ("GET", "/solvers", _) => Reply::fixture("solvers"),
            ("GET", "/health", _) => Reply::fixture("health"),
            (_, _, "largest") => Reply::fixture("run_returned"),
            (_, _, "boom" | "importfail") => Reply::fixture("run_raised"),
            (_, _, "spin") => Reply::fixture("run_timeout"),
            (_, _, "oom") => limit("memory"),
            (_, _, "flood") => limit("output"),
            (_, _, "cpu") => limit("cpu"),
            (_, _, "crash") => limit("crashed"),
            (_, _, "floaty") => {
                let mut body = fixture("run_returned");
                body["value"] = json!({"x": 1.5});
                Reply::ok(body)
            }
            (_, _, "notjson") => {
                let mut body = fixture("run_raised");
                body["exception"]["type"] = json!("NotJsonSerializable");
                Reply::ok(body)
            }
            (_, _, "ghost") => Reply::error(
                400,
                "invalid_input",
                "work.sunflower has no function named 'ghost' (entry)",
            ),
            (_, _, "gone") => Reply::error(404, "not_found", "python/work/x.py does not exist"),
            (_, _, "unreachable") => Reply::error(500, "internal", "boom"),
            (_, _, "garbage") => Reply::ok(json!({"nonsense": true})),
            (_, _, "stale") => Reply::fixture("run_returned").with_file_sha("deadbeef"),
            (_, _, "slow") => Reply::fixture("run_returned").delayed(10_000),
            (_, _, "truncated") => {
                let mut body = fixture("run_returned");
                body["stdout_truncated"] = json!(true);
                body["stderr"] = json!("warn\n");
                body["stderr_truncated"] = json!(true);
                Reply::ok(body)
            }
            (_, _, "prime_value") => Reply::fixture("search_found"),
            (_, _, "pair") => {
                let mut body = fixture("search_found");
                body["search"] = json!({
                    "checked": 6, "witness": [1, 2], "witness_json": "[1,2]",
                    "witness_tuples": [[]],
                });
                Reply::ok(body)
            }
            (_, _, "noisy") => {
                let mut body = fixture("search_found");
                body["stdout"] = json!("lots\n");
                body["stdout_truncated"] = json!(true);
                body["stderr_truncated"] = json!(true);
                Reply::ok(body)
            }
            (_, _, "stale_search") => Reply::fixture("search_found").with_file_sha("deadbeef"),
            (_, _, "always") => Reply::fixture("search_exhausted"),
            (_, _, "broken") => Reply::fixture("search_predicate_raised"),
            (_, _, "capped") => {
                let mut body = fixture("search_exhausted");
                body["result"] = json!("capped");
                Reply::ok(body)
            }
            (_, _, "timed") => search_limit(Some(77)),
            (_, _, "timed_blind") => search_limit(None),
            _ => Reply::error(500, "internal", "unrouted"),
        };
        if reply.body.get("file_sha256").is_some() && reply.body["file_sha256"] != "deadbeef" {
            reply.with_file_sha(&sha)
        } else {
            reply
        }
    }
}

struct Rig {
    setup: Setup,
    stub: Stub,
}

async fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let stub = Stub::start(routes(dir.path().to_path_buf())).await;
    let setup = Setup::in_dir(dir, Some(&stub.base));
    setup.write("python/work/sunflower.py", SUNFLOWER);
    setup.write("python/work/euler.py", EULER);
    Rig { setup, stub }
}

fn data(envelope: &Envelope) -> &Value {
    &envelope.data
}

async fn run(rig: &Rig, args: Value) -> Envelope {
    rig.setup.tool("py_run").call(&ctx(), args).await
}

async fn search(rig: &Rig, args: Value) -> Envelope {
    rig.setup
        .tool("py_search_counterexample")
        .call(&ctx(), args)
        .await
}

fn run_args(entry: &str) -> Value {
    json!({"path": "python/work/sunflower.py", "entry": entry, "args": {"n": 6, "k": 3}})
}

fn search_args(predicate: &str) -> Value {
    json!({
        "path": "python/work/euler.py",
        "predicate": predicate,
        "frame": {"kind": "range", "from": 0, "to": 100},
    })
}

// --- the contract ---------------------------------------------------------

#[tokio::test]
async fn py_run_passes_the_contract_checker() {
    let rig = rig().await;
    let fixtures = vec![
        Fixture::with_ctx(ctx(), run_args("largest")),
        Fixture::with_ctx(ctx(), run_args("boom")),
        Fixture::with_ctx(
            ctx(),
            json!({"path": "lean/Supermath/A.lean", "entry": "f"}),
        ),
        Fixture::with_ctx(
            ctx(),
            json!({"path": "python/work/absent.py", "entry": "f"}),
        ),
        Fixture::with_ctx(
            ctx(),
            json!({"path": "python/work/sunflower.py", "entry": "f", "timeout_s": 100_000}),
        ),
        Fixture::with_ctx(ctx(), run_args("unreachable")),
    ];
    contract::check(rig.setup.tool("py_run").as_ref(), "py", &fixtures)
        .await
        .unwrap();
}

#[tokio::test]
async fn py_search_counterexample_passes_the_contract_checker() {
    let rig = rig().await;
    let mut huge = search_args("always");
    huge["frame"] = json!({"kind": "range", "from": 0, "to": 5_000_000});
    let mut bad = search_args("always");
    bad["frame"] = json!({"kind": "range", "from": 9, "to": 0});
    let mut missing = search_args("always");
    missing["path"] = json!("python/work/absent.py");
    let fixtures = vec![
        Fixture::with_ctx(ctx(), search_args("prime_value")),
        Fixture::with_ctx(ctx(), search_args("always")),
        Fixture::with_ctx(ctx(), bad),
        Fixture::with_ctx(ctx(), missing),
        Fixture::with_ctx(ctx(), huge),
        Fixture::with_ctx(ctx(), search_args("unreachable")),
    ];
    contract::check(
        rig.setup.tool("py_search_counterexample").as_ref(),
        "py",
        &fixtures,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn py_solvers_passes_the_contract_checker() {
    let setup = Setup::new(None);
    let fixtures = vec![
        Fixture::with_ctx(ctx(), json!({})),
        Fixture::with_ctx(ctx(), json!({"area": "alchemy"})),
    ];
    contract::check(setup.tool("py_solvers").as_ref(), "py", &fixtures)
        .await
        .unwrap();
}

// --- py_run ---------------------------------------------------------------

#[tokio::test]
async fn a_returned_run_carries_the_value_and_complete_provenance() {
    let rig = rig().await;
    let answer = run(&rig, run_args("largest")).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    let d = data(&answer);
    assert_eq!(d["result"], "returned");
    assert_eq!(d["value"]["size"], 18);
    assert_eq!(d["exact"], true);
    assert_eq!(d["stdout"], "searched 6 3\n");

    let provenance = &d["provenance"];
    let path = ProjectPath::parse("python/work/sunflower.py").unwrap();
    assert_eq!(provenance["path"], "python/work/sunflower.py");
    assert_eq!(
        provenance["file_sha256"],
        json!(rig.setup.workspace.sha256("demo", &path).unwrap())
    );
    assert_eq!(
        provenance["imports_sha256"],
        json!(
            rig.setup
                .workspace
                .imports_digest("demo", &path)
                .unwrap()
                .sha256
        )
    );
    assert_eq!(provenance["entry"], "largest");
    assert_eq!(
        provenance["args_sha256"],
        json!(args_sha256(&json!({"k": 3, "n": 6})))
    );
    assert_eq!(provenance["seed"], 0);
    assert_eq!(provenance["python"], "3.12.15");
    assert_eq!(provenance["lock_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(provenance["elapsed_ms"], 121);
    assert_eq!(provenance["peak_rss_mb"], 26);

    assert!(answer.next[0].starts_with("ledger_record"));
    assert!(answer.next[0].contains("\"call_id\": \"call-1\""));
    assert_eq!(answer.unknowns.len(), 0);
}

#[tokio::test]
async fn the_request_the_service_sees_matches_the_arguments() {
    let rig = rig().await;
    let mut args = run_args("largest");
    args["seed"] = json!(7);
    args["timeout_s"] = json!(25);
    run(&rig, args).await;
    let seen = &rig.stub.requests()[0];
    assert_eq!(seen.method, "POST");
    assert_eq!(seen.path, "/run");
    assert_eq!(
        seen.body,
        json!({
            "project": "demo", "path": "python/work/sunflower.py", "entry": "largest",
            "args": {"n": 6, "k": 3}, "seed": 7, "timeout_s": 25
        })
    );
}

#[tokio::test]
async fn a_default_timeout_is_filled_in_from_the_configuration() {
    let rig = rig().await;
    run(&rig, run_args("largest")).await;
    assert_eq!(rig.stub.requests()[0].body["timeout_s"], 30);
}

#[tokio::test]
async fn a_raised_exception_is_an_ok_result() {
    let rig = rig().await;
    let answer = run(&rig, run_args("boom")).await;
    assert_eq!(answer.outcome, Outcome::Ok);
    assert_eq!(data(&answer)["result"], "raised");
    assert_eq!(data(&answer)["exception"]["type"], "ValueError");
    assert_eq!(data(&answer)["exception"]["message"], "nope");
    assert_eq!(data(&answer)["provenance"]["entry"], "boom");
}

#[tokio::test]
async fn a_value_that_is_not_json_is_reported_as_a_raised_result() {
    let rig = rig().await;
    let answer = run(&rig, run_args("notjson")).await;
    assert_eq!(data(&answer)["result"], "raised");
    assert_eq!(data(&answer)["exception"]["type"], "NotJsonSerializable");
}

#[tokio::test]
async fn every_limit_is_unknown_with_the_output_so_far() {
    let rig = rig().await;
    for (entry, limit) in [
        ("spin", "time"),
        ("oom", "memory"),
        ("flood", "output"),
        ("cpu", "cpu"),
        ("crash", "crashed"),
    ] {
        let answer = run(&rig, run_args(entry)).await;
        assert_eq!(answer.outcome, Outcome::Unknown, "{entry}");
        assert_eq!(data(&answer)["result"], "limit");
        assert_eq!(data(&answer)["limit"], limit);
        assert_eq!(data(&answer)["stdout"], "started\n");
        assert!(answer.unknowns[0].contains(limit), "{:?}", answer.unknowns);
        assert!(answer.unknowns[0].contains("says nothing about the mathematics"));
        assert!(answer.code().is_none());
    }
}

#[tokio::test]
async fn a_float_result_not_declared_exact_says_so() {
    let rig = rig().await;
    let answer = run(&rig, run_args("floaty")).await;
    assert_eq!(answer.outcome, Outcome::Ok);
    assert_eq!(data(&answer)["exact"], false);
    assert!(answer.unknowns[0].contains("not declared exact"));
}

#[tokio::test]
async fn runs_are_logged_for_file_list_with_the_digest_they_were_for() {
    let rig = rig().await;
    let observer = rig.setup.python.observer();
    let path = ProjectPath::parse("python/work/sunflower.py").unwrap();
    assert!(observer.last_result("demo", &path).is_none());

    run(&rig, run_args("largest")).await;
    let sha = rig.setup.workspace.sha256("demo", &path).unwrap();
    let last = observer.last_result("demo", &path).unwrap();
    assert_eq!(
        (last.result.as_str(), last.sha256.as_str()),
        ("returned", sha.as_str())
    );

    run(&rig, run_args("boom")).await;
    assert_eq!(
        observer.last_result("demo", &path).unwrap().result,
        "raised"
    );
    run(&rig, run_args("spin")).await;
    assert_eq!(
        observer.last_result("demo", &path).unwrap().result,
        "unknown"
    );
}

#[tokio::test]
async fn invalid_arguments_are_refused_before_the_service_is_called() {
    let rig = rig().await;
    let cases = [
        json!({"path": "python/work/sunflower.py"}),
        json!({"path": "python/work/sunflower.py", "entry": "f", "bogus": 1}),
        json!({"path": "python/work/sunflower.py", "entry": "not a name"}),
        json!({"path": "python/work/sunflower.py", "entry": "f", "args": [1]}),
        json!({"path": "python/work/sunflower.py", "entry": "f", "timeout_s": 0}),
        json!({"path": "python/work/sunflower.py", "entry": "f", "args": {"big": "x".repeat(70_000)}}),
        json!({"path": "../etc/passwd", "entry": "f"}),
        json!({"path": "lean/Supermath/A.lean", "entry": "f"}),
    ];
    for case in cases {
        let answer = run(&rig, case.clone()).await;
        assert_eq!(answer.code(), Some(ErrorCode::InvalidInput), "{case}");
    }
    assert!(rig.stub.requests().is_empty());
}

#[tokio::test]
async fn null_args_mean_no_arguments() {
    let rig = rig().await;
    let mut args = run_args("largest");
    args["args"] = Value::Null;
    run(&rig, args).await;
    assert_eq!(rig.stub.requests()[0].body["args"], json!({}));
}

#[tokio::test]
async fn a_missing_file_is_not_found_and_a_long_timeout_is_over_budget() {
    let rig = rig().await;
    let answer = run(&rig, json!({"path": "python/work/absent.py", "entry": "f"})).await;
    assert_eq!(answer.code(), Some(ErrorCode::NotFound));
    let mut args = run_args("largest");
    args["timeout_s"] = json!(301);
    assert_eq!(
        run(&rig, args).await.code(),
        Some(ErrorCode::BudgetExceeded)
    );
}

#[tokio::test]
async fn the_calls_wall_time_budget_bounds_the_timeout() {
    let rig = rig().await;
    let mut tight = ctx();
    tight.budget = Budget {
        wall_time_ms: 5_000,
        ..Budget::default()
    };
    let mut args = run_args("largest");
    args["timeout_s"] = json!(10);
    let refused = rig.setup.tool("py_run").call(&tight, args).await;
    assert_eq!(refused.code(), Some(ErrorCode::BudgetExceeded));
    let defaulted = rig
        .setup
        .tool("py_run")
        .call(&tight, run_args("largest"))
        .await;
    assert_eq!(defaulted.outcome, Outcome::Ok);
    assert_eq!(rig.stub.requests()[0].body["timeout_s"], 5);
}

#[tokio::test]
async fn a_spent_compute_budget_is_refused_before_the_service_is_called() {
    let rig = rig().await;
    let mut spent = ctx();
    spent.budget = Budget {
        wall_time_ms: 0,
        ..Budget::default()
    };
    let answer = rig
        .setup
        .tool("py_run")
        .call(&spent, run_args("largest"))
        .await;
    assert_eq!(answer.code(), Some(ErrorCode::BudgetExceeded), "{answer:?}");
    let searched = rig
        .setup
        .tool("py_search_counterexample")
        .call(&spent, search_args("always"))
        .await;
    assert_eq!(searched.code(), Some(ErrorCode::BudgetExceeded));
    assert!(rig.stub.requests().is_empty());
}

#[tokio::test]
async fn service_refusals_map_onto_the_error_codes() {
    let rig = rig().await;
    let cases = [
        ("ghost", ErrorCode::InvalidInput),
        ("gone", ErrorCode::NotFound),
        ("unreachable", ErrorCode::BackendUnavailable),
        ("garbage", ErrorCode::BackendUnavailable),
        ("stale", ErrorCode::BackendUnavailable),
    ];
    for (entry, code) in cases {
        let answer = run(&rig, run_args(entry)).await;
        assert_eq!(answer.code(), Some(code), "{entry}: {answer:?}");
    }
}

#[tokio::test]
async fn a_service_that_ran_a_different_file_version_is_reported() {
    let rig = rig().await;
    let answer = run(&rig, run_args("stale")).await;
    assert!(answer.error.unwrap().message.contains("different version"));
}

#[tokio::test]
async fn without_a_configured_service_every_tool_is_backend_unavailable() {
    let setup = Setup::new(None);
    setup.write("python/work/sunflower.py", SUNFLOWER);
    let answer = setup.tool("py_run").call(&ctx(), run_args("largest")).await;
    assert_eq!(answer.code(), Some(ErrorCode::BackendUnavailable));
    assert!(answer.error.unwrap().message.contains("PYTHON_URL"));
    let solvers = setup.tool("py_solvers").call(&ctx(), json!({})).await;
    assert_eq!(solvers.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn an_unreachable_service_is_backend_unavailable() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let setup = Setup::new(Some(&url));
    setup.write("python/work/sunflower.py", SUNFLOWER);
    let answer = setup.tool("py_run").call(&ctx(), run_args("largest")).await;
    assert_eq!(answer.code(), Some(ErrorCode::BackendUnavailable));
}

#[tokio::test]
async fn a_cancelled_call_stops_waiting_for_the_service() {
    let rig = rig().await;
    let context = ctx();
    let cancel = context.cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancel.cancel();
    });
    let started = std::time::Instant::now();
    let answer = rig
        .setup
        .tool("py_run")
        .call(&context, run_args("slow"))
        .await;
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert!(answer.unknowns[0].contains("cancelled"));
}

// --- py_search_counterexample ---------------------------------------------

#[tokio::test]
async fn a_found_witness_is_written_to_a_generated_file() {
    let rig = rig().await;
    let answer = search(&rig, search_args("prime_value")).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    let d = data(&answer);
    assert_eq!(d["result"], "found");
    assert_eq!(d["witness"], 40);
    assert_eq!(d["checked_before"], 40);
    assert_eq!(d["frame"], json!({"kind": "range", "from": 0, "to": 100}));

    let relative = d["witness_file"]["path"].as_str().unwrap();
    assert!(relative.starts_with("python/work/witnesses/witness_prime_value_"));
    let path = ProjectPath::parse(relative).unwrap();
    let file = rig.setup.workspace.read("demo", &path).unwrap();
    assert_eq!(d["witness_file"]["sha256"], json!(file.sha256));
    assert!(file.text.contains("from work.euler import prime_value"));
    assert!(file.text.contains("WITNESS = json.loads(\"40\")"));
    // One concrete call, with arguments: re-run the witness file's check().
    assert_eq!(answer.next.len(), 1, "{:?}", answer.next);
    let (tool, arguments) = answer.next[0].split_once(' ').unwrap();
    assert_eq!(tool, "py_run");
    let arguments: Value = serde_json::from_str(arguments).unwrap();
    assert_eq!(arguments, json!({"path": relative, "entry": "check"}));

    let log = rig.setup.workspace.write_log("demo").unwrap();
    let written = log.for_call("call-1").unwrap();
    assert_eq!(written.len(), 1);
    assert_eq!(written[0].path, relative);
    assert_eq!(written[0].tool, "py_search_counterexample");
}

#[tokio::test]
async fn finding_the_same_witness_twice_is_not_an_error() {
    let rig = rig().await;
    let first = search(&rig, search_args("prime_value")).await;
    let second = search(&rig, search_args("prime_value")).await;
    assert_eq!(second.outcome, Outcome::Ok, "{second:?}");
    assert_eq!(data(&first)["witness_file"], data(&second)["witness_file"]);
}

#[tokio::test]
async fn the_search_request_states_the_frame_and_the_cap() {
    let rig = rig().await;
    let mut args = search_args("prime_value");
    args["frame"] =
        json!({"kind": "product", "ranges": [{"from": 0, "to": 3}, {"from": 1, "to": 2}]});
    args["seed"] = json!(5);
    args["timeout_s"] = json!(20);
    search(&rig, args).await;
    let seen = &rig.stub.requests()[0];
    assert_eq!(seen.path, "/search");
    assert_eq!(seen.body["frame"]["kind"], "product");
    assert_eq!(seen.body["max_frame"], 1_000_000);
    assert_eq!(seen.body["seed"], 5);
    assert_eq!(seen.body["timeout_s"], 20);
    assert_eq!(seen.body["predicate"], "prime_value");
}

#[tokio::test]
async fn an_exhausted_frame_is_ok_and_says_how_far_it_looked() {
    let rig = rig().await;
    let answer = search(&rig, search_args("always")).await;
    assert_eq!(answer.outcome, Outcome::Ok);
    let d = data(&answer);
    assert_eq!(d["result"], "exhausted");
    assert_eq!(d["checked"], 1000);
    assert_eq!(d["frame_description"], "every integer from 0 to 100");
    let coverage = answer.coverage.unwrap();
    assert_eq!(coverage.searched, ["every integer from 0 to 100"]);
    assert_eq!(coverage.not_covered, ["values outside the frame"]);
    assert_eq!(answer.unknowns.len(), 0);
    assert!(answer.next[0].contains("\"kind\": \"bound\""));
    assert!(answer.next[0].contains("\"tool\": \"py_search_counterexample\""));
    assert!(answer.next[0].contains("\"frame\": \"every integer from 0 to 100\""));
    let last = rig
        .setup
        .python
        .observer()
        .last_result("demo", &ProjectPath::parse("python/work/euler.py").unwrap())
        .unwrap();
    assert_eq!(last.result, "exhausted");
}

#[tokio::test]
async fn a_predicate_that_raises_is_not_a_counterexample() {
    let rig = rig().await;
    let answer = search(&rig, search_args("broken")).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    let d = data(&answer);
    assert_eq!(d["result"], "predicate_raised");
    assert_eq!(d["input"], 2);
    assert_eq!(d["exception"]["type"], "ZeroDivisionError");
    assert!(answer.unknowns[0].contains("not a counterexample"));
}

#[tokio::test]
async fn a_capped_search_and_a_limit_report_the_part_covered() {
    let rig = rig().await;
    let capped = search(&rig, search_args("capped")).await;
    assert_eq!(capped.outcome, Outcome::Unknown);
    assert_eq!(data(&capped)["result"], "capped");
    assert_eq!(data(&capped)["checked"], 1000);

    let timed = search(&rig, search_args("timed")).await;
    assert_eq!(timed.outcome, Outcome::Unknown);
    assert_eq!(data(&timed)["limit"], "time");
    assert_eq!(data(&timed)["checked"], 77);
    assert_eq!(data(&timed)["covered"], json!({"from": 0, "to": 76}));
    assert!(timed.unknowns[0].contains("77 values were checked"));

    let blind = search(&rig, search_args("timed_blind")).await;
    assert!(blind.unknowns[0].contains("not known"));
    assert_eq!(data(&blind)["covered"], Value::Null);
}

#[tokio::test]
async fn a_file_that_raises_before_searching_is_unknown() {
    let rig = rig().await;
    let mut args = search_args("importfail");
    args["predicate"] = json!("importfail");
    let answer = search(&rig, args).await;
    assert_eq!(answer.outcome, Outcome::Unknown);
    assert_eq!(data(&answer)["result"], "raised");
    assert_eq!(data(&answer)["exception"]["type"], "ValueError");
}

#[tokio::test]
async fn bad_frames_and_huge_frames_are_refused_before_the_service_is_called() {
    let rig = rig().await;
    let mut empty = search_args("always");
    empty["frame"] = json!({"kind": "range", "from": 5, "to": 1});
    assert_eq!(
        search(&rig, empty).await.code(),
        Some(ErrorCode::InvalidInput)
    );
    let mut unbounded = search_args("always");
    unbounded["frame"] = json!({"kind": "range", "from": 0});
    assert_eq!(
        search(&rig, unbounded).await.code(),
        Some(ErrorCode::InvalidInput)
    );
    let mut huge = search_args("always");
    huge["frame"] = json!({"kind": "range", "from": 0, "to": 5_000_000});
    assert_eq!(
        search(&rig, huge).await.code(),
        Some(ErrorCode::BudgetExceeded)
    );
    let mut product = search_args("always");
    product["frame"] =
        json!({"kind": "product", "ranges": [{"from": 0, "to": 5000}, {"from": 0, "to": 5000}]});
    assert_eq!(
        search(&rig, product).await.code(),
        Some(ErrorCode::BudgetExceeded)
    );
    let mut name = search_args("always");
    name["predicate"] = json!("not a name");
    assert_eq!(
        search(&rig, name).await.code(),
        Some(ErrorCode::InvalidInput)
    );
    assert!(rig.stub.requests().is_empty());
}

#[tokio::test]
async fn a_generator_frame_is_admitted_by_the_service_not_by_size() {
    let rig = rig().await;
    let mut args = search_args("always");
    args["frame"] = json!({"kind": "generator", "entry": "values"});
    let answer = search(&rig, args).await;
    assert_eq!(data(&answer)["result"], "exhausted");
}

#[tokio::test]
async fn an_unwritable_witness_is_reported_without_losing_the_witness() {
    let rig = rig().await;
    let work = rig.setup.dir.path().join("demo/python/work");
    std::fs::write(work.join("witnesses"), "in the way").unwrap();
    let answer = search(&rig, search_args("prime_value")).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(data(&answer)["witness"], 40);
    assert_eq!(data(&answer)["witness_file"], Value::Null);
    assert!(answer.unknowns[0].contains("could not be written"));
    assert!(answer.next.is_empty(), "{:?}", answer.next);
}

fn product_frame() -> Value {
    json!({"kind": "product", "ranges": [{"from": 0, "to": 2}, {"from": 0, "to": 3}]})
}

fn generator_frame() -> Value {
    json!({"kind": "generator", "entry": "values"})
}

fn witness_source(rig: &Rig, answer: &Envelope) -> String {
    let relative = data(answer)["witness_file"]["path"].as_str().unwrap();
    rig.setup
        .workspace
        .read("demo", &ProjectPath::parse(relative).unwrap())
        .unwrap()
        .text
}

#[tokio::test]
async fn a_product_frame_witness_is_written_back_as_a_tuple() {
    let rig = rig().await;
    let mut args = search_args("pair");
    args["frame"] = product_frame();
    let answer = search(&rig, args).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(data(&answer)["witness"], json!([1, 2]));
    assert_eq!(data(&answer)["frame"]["kind"], "product");
    assert!(witness_source(&rig, &answer).contains("WITNESS = tuple(json.loads(\"[1,2]\"))"));
}

#[tokio::test]
async fn a_generator_that_yields_tuples_keeps_the_tuple_in_its_witness_file() {
    let rig = rig().await;
    let mut args = search_args("pair");
    args["frame"] = generator_frame();
    let answer = search(&rig, args).await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    // JSON turned the tuple into a list; check() must see the tuple again.
    assert!(witness_source(&rig, &answer).contains("WITNESS = tuple(json.loads(\"[1,2]\"))"));
}

#[tokio::test]
async fn an_exhausted_product_frame_is_ok_and_names_the_box() {
    let rig = rig().await;
    let mut args = search_args("always");
    args["frame"] = product_frame();
    let answer = search(&rig, args).await;
    assert_eq!(answer.outcome, Outcome::Ok);
    assert_eq!(data(&answer)["result"], "exhausted");
    let description = data(&answer)["frame_description"]
        .as_str()
        .unwrap()
        .to_owned();
    let coverage = answer.coverage.unwrap();
    assert_eq!(coverage.searched, [description]);
    assert_eq!(coverage.searched, ["every tuple in [0, 2] x [0, 3]"]);
}

#[tokio::test]
async fn limits_and_caps_on_product_and_generator_frames_are_unknown() {
    let rig = rig().await;
    for frame in [product_frame(), generator_frame()] {
        let mut args = search_args("timed");
        args["frame"] = frame.clone();
        let timed = search(&rig, args).await;
        assert_eq!(timed.outcome, Outcome::Unknown, "{frame}");
        assert_eq!(data(&timed)["result"], "limit");
        assert_eq!(data(&timed)["checked"], 77);
        // Only a range has a simple covered prefix.
        assert_eq!(data(&timed)["covered"], Value::Null, "{frame}");

        let mut args = search_args("capped");
        args["frame"] = frame.clone();
        let capped = search(&rig, args).await;
        assert_eq!(capped.outcome, Outcome::Unknown, "{frame}");
        assert_eq!(data(&capped)["result"], "capped");
        assert_eq!(data(&capped)["checked"], 1000);
        assert!(capped.code().is_none());
    }
}

#[tokio::test]
async fn search_refusals_and_a_stale_file_map_onto_the_error_codes() {
    let rig = rig().await;
    for (predicate, code) in [
        ("ghost", ErrorCode::InvalidInput),
        ("gone", ErrorCode::NotFound),
        ("unreachable", ErrorCode::BackendUnavailable),
        ("stale_search", ErrorCode::BackendUnavailable),
    ] {
        let answer = search(&rig, search_args(predicate)).await;
        assert_eq!(answer.code(), Some(code), "{predicate}: {answer:?}");
    }
    let stale = search(&rig, search_args("stale_search")).await;
    assert!(stale.error.unwrap().message.contains("different version"));
}

#[tokio::test]
async fn output_truncation_is_passed_through_by_run_and_search() {
    let rig = rig().await;
    let ran = run(&rig, run_args("truncated")).await;
    assert_eq!(data(&ran)["stdout_truncated"], true);
    assert_eq!(data(&ran)["stderr"], "warn\n");
    assert_eq!(data(&ran)["stderr_truncated"], true);
    let plain = run(&rig, run_args("largest")).await;
    assert_eq!(data(&plain)["stdout_truncated"], false);
    assert_eq!(data(&plain)["stderr_truncated"], false);

    let searched = search(&rig, search_args("noisy")).await;
    assert_eq!(data(&searched)["stdout"], "lots\n");
    assert_eq!(data(&searched)["stdout_truncated"], true);
    assert_eq!(data(&searched)["stderr_truncated"], true);
    let exhausted = search(&rig, search_args("always")).await;
    assert_eq!(data(&exhausted)["stdout_truncated"], false);
    assert_eq!(data(&exhausted)["stderr_truncated"], false);
}

// --- py_solvers -----------------------------------------------------------

#[tokio::test]
async fn solvers_are_listed_with_the_lock_digest_as_coverage() {
    let rig = rig().await;
    let answer = rig.setup.tool("py_solvers").call(&ctx(), json!({})).await;
    assert_eq!(answer.outcome, Outcome::Ok);
    let rows = data(&answer)["solvers"].as_array().unwrap();
    assert_eq!(rows.len(), 14);
    let sympy = rows.iter().find(|r| r["package"] == "sympy").unwrap();
    assert_eq!(sympy["import_name"], "sympy");
    assert_eq!(sympy["area"], "symbolic");
    assert_ne!(sympy["version"].as_str().unwrap(), "");
    let lock = data(&answer)["lock_sha256"].as_str().unwrap().to_owned();
    assert!(answer.coverage.unwrap().searched[0].contains(&lock));
    assert_eq!(rig.stub.requests()[0].method, "GET");
}

#[tokio::test]
async fn solvers_can_be_filtered_by_area() {
    let rig = rig().await;
    let answer = rig
        .setup
        .tool("py_solvers")
        .call(&ctx(), json!({"area": "optimization"}))
        .await;
    let packages: Vec<&str> = data(&answer)["solvers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["package"].as_str().unwrap())
        .collect();
    assert_eq!(packages, ["ortools", "highspy", "cvxpy"]);
}

#[tokio::test]
async fn an_unknown_area_is_invalid_input_and_an_empty_one_is_unknown() {
    let rig = rig().await;
    let bad = rig
        .setup
        .tool("py_solvers")
        .call(&ctx(), json!({"area": "alchemy"}))
        .await;
    assert_eq!(bad.code(), Some(ErrorCode::InvalidInput));

    let stub = Stub::start(|_| Reply::ok(json!({"python": "3.12", "solvers": []}))).await;
    let setup = Setup::new(Some(&stub.base));
    let empty = setup
        .tool("py_solvers")
        .call(&ctx(), json!({"area": "sat"}))
        .await;
    assert_eq!(empty.outcome, Outcome::Ok);
    assert!(empty.unknowns[0].contains("no solver"));
    assert!(empty.coverage.unwrap().searched[0].contains("digest unavailable"));
}

#[tokio::test]
async fn solvers_are_backend_unavailable_on_a_server_error_or_a_garbage_body() {
    for reply in [
        Reply::error(500, "internal", "cannot read the solver list"),
        Reply::ok(json!({"nonsense": true})),
        Reply::ok(json!("not even an object")),
    ] {
        let body = reply.body.clone();
        let stub = Stub::start(move |_| reply.clone()).await;
        let setup = Setup::new(Some(&stub.base));
        let answer = setup.tool("py_solvers").call(&ctx(), json!({})).await;
        assert_eq!(
            answer.code(),
            Some(ErrorCode::BackendUnavailable),
            "{body}: {answer:?}"
        );
    }
}

#[tokio::test]
async fn the_client_reads_health() {
    let rig = rig().await;
    let health = rig.setup.python.client().health().await.unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.python, "3.12.15");
    assert_eq!(health.lock_sha256.unwrap().len(), 64);
}

#[tokio::test]
async fn each_call_uses_its_own_call_id_in_next() {
    let rig = rig().await;
    let answer = rig
        .setup
        .tool("py_run")
        .call(&ctx_with("call-77"), run_args("largest"))
        .await;
    assert!(answer.next[0].contains("call-77"));
    let _: CallContext = ctx();
}
