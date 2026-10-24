//! Tests for the answer decoding and the unconfigured client.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};

use super::types::Answer;
use super::{Limit, PythonClient, RunOutcome, RunRequest, SearchOutcome, SearchRequest};
use crate::{Error, PythonConfig};

fn answer(mut fields: Value) -> Answer {
    let base = json!({
        "python": "3.12.1", "lock_sha256": "lock", "file_sha256": "file",
        "elapsed_ms": 40, "peak_rss_mb": 12,
        "stdout": "hi\n", "stdout_truncated": false, "stderr": "", "stderr_truncated": true
    });
    for (key, value) in base.as_object().unwrap() {
        fields[key] = value.clone();
    }
    serde_json::from_value(fields).unwrap()
}

fn exception() -> Value {
    json!({"type": "ValueError", "message": "nope", "traceback": ["a.py:3 in f: raise"]})
}

#[test]
fn a_returned_answer_carries_value_and_execution() {
    let report = answer(json!({"result": "returned", "value": {"size": 20}}))
        .into_run()
        .unwrap();
    assert_eq!(
        report.outcome,
        RunOutcome::Returned {
            value: json!({"size": 20})
        }
    );
    assert_eq!(report.execution.python, "3.12.1");
    assert_eq!(report.execution.lock_sha256.as_deref(), Some("lock"));
    assert_eq!(report.execution.file_sha256, "file");
    assert_eq!(report.execution.elapsed_ms, 40);
    assert_eq!(report.execution.peak_rss_mb, Some(12));
    assert_eq!(report.execution.output.stdout, "hi\n");
    assert!(report.execution.output.stderr_truncated);
}

#[test]
fn a_raised_answer_carries_the_exception() {
    let report = answer(json!({"result": "raised", "exception": exception()}))
        .into_run()
        .unwrap();
    let RunOutcome::Raised { exception } = report.outcome else {
        panic!("not raised");
    };
    assert_eq!(exception.type_name, "ValueError");
    assert_eq!(exception.traceback.len(), 1);
}

#[test]
fn every_limit_name_decodes() {
    for (name, limit) in [
        ("time", Limit::Time),
        ("cpu", Limit::Cpu),
        ("memory", Limit::Memory),
        ("output", Limit::Output),
        ("crashed", Limit::Crashed),
    ] {
        let report = answer(json!({"result": "limit", "limit": name}))
            .into_run()
            .unwrap();
        assert_eq!(report.outcome, RunOutcome::Limit { limit });
        assert_eq!(limit.as_str(), name);
        assert_ne!(limit.explanation(), "");
    }
}

#[test]
fn run_answers_that_do_not_fit_the_api_are_decode_errors() {
    let cases = [
        json!({"result": "exhausted"}),
        json!({"result": "raised"}),
        json!({"result": "limit"}),
    ];
    for case in cases {
        assert!(matches!(answer(case).into_run(), Err(Error::Decode(_))));
    }
    let no_digest: Answer =
        serde_json::from_value(json!({"result": "returned", "python": "3.12", "elapsed_ms": 1}))
            .unwrap();
    assert!(matches!(no_digest.into_run(), Err(Error::Decode(_))));
}

#[test]
fn a_found_search_carries_the_witness_text() {
    let report = answer(json!({
        "result": "found",
        "search": {"checked": 41, "witness": "1180591620717411303424", "witness_json": "1180591620717411303424"}
    }))
    .into_search()
    .unwrap();
    assert_eq!(
        report.outcome,
        SearchOutcome::Found {
            witness: json!("1180591620717411303424"),
            witness_json: "1180591620717411303424".into(),
            witness_tuples: Vec::new(),
            checked: 41,
        }
    );
}

#[test]
fn a_found_search_carries_where_the_witness_held_tuples() {
    let report = answer(json!({
        "result": "found",
        "search": {"checked": 2, "witness": [1, [2]], "witness_json": "[1,[2]]",
                   "witness_tuples": [[], [1]]}
    }))
    .into_search()
    .unwrap();
    let SearchOutcome::Found { witness_tuples, .. } = report.outcome else {
        panic!("expected found, got {:?}", report.outcome);
    };
    assert_eq!(witness_tuples, [json!([]), json!([1])]);
}

#[test]
fn the_other_search_results_decode() {
    let exhausted = answer(json!({"result": "exhausted", "search": {"checked": 9}}))
        .into_search()
        .unwrap();
    assert_eq!(exhausted.outcome, SearchOutcome::Exhausted { checked: 9 });

    let capped = answer(json!({"result": "capped", "search": {"checked": 5}}))
        .into_search()
        .unwrap();
    assert_eq!(capped.outcome, SearchOutcome::Capped { checked: 5 });

    let raised = answer(json!({"result": "predicate_raised", "search": {"checked": 3, "input": 3, "exception": exception()}}))
        .into_search()
        .unwrap();
    assert!(matches!(
        raised.outcome,
        SearchOutcome::PredicateRaised { checked: 3, .. }
    ));

    let import = answer(json!({"result": "raised", "exception": exception()}))
        .into_search()
        .unwrap();
    assert!(matches!(import.outcome, SearchOutcome::Raised { .. }));

    let limit = answer(json!({"result": "limit", "limit": "time", "search": {"checked": 77}}))
        .into_search()
        .unwrap();
    assert_eq!(
        limit.outcome,
        SearchOutcome::Limit {
            limit: Limit::Time,
            checked: Some(77)
        }
    );

    let bare = answer(json!({"result": "limit", "limit": "memory"}))
        .into_search()
        .unwrap();
    assert_eq!(
        bare.outcome,
        SearchOutcome::Limit {
            limit: Limit::Memory,
            checked: None
        }
    );
}

#[test]
fn search_answers_that_do_not_fit_the_api_are_decode_errors() {
    let cases = [
        json!({"result": "returned"}),
        json!({"result": "found", "search": {"checked": 1}}),
        json!({"result": "found", "search": {"witness_json": "1"}}),
        json!({"result": "exhausted"}),
        json!({"result": "predicate_raised", "search": {"checked": 1}}),
        json!({"result": "raised"}),
        json!({"result": "limit"}),
    ];
    for case in cases {
        assert!(matches!(answer(case).into_search(), Err(Error::Decode(_))));
    }
}

#[test]
fn wire_forms_of_requests_are_pinned() {
    let run = RunRequest {
        project: "demo".into(),
        root: None,
        path: "python/work/a.py".into(),
        entry: "go".into(),
        args: json!({"n": 6}),
        seed: 1,
        timeout_s: 60,
    };
    assert_eq!(
        serde_json::to_value(&run).unwrap(),
        json!({"project": "demo", "path": "python/work/a.py", "entry": "go",
               "args": {"n": 6}, "seed": 1, "timeout_s": 60})
    );
    let search = SearchRequest {
        project: "demo".into(),
        root: None,
        path: "python/work/a.py".into(),
        predicate: "p".into(),
        frame: json!({"kind": "range", "from": 0, "to": 9}),
        seed: 0,
        timeout_s: 30,
        max_frame: 100,
    };
    assert_eq!(
        serde_json::to_value(&search).unwrap(),
        json!({"project": "demo", "path": "python/work/a.py", "predicate": "p",
               "frame": {"kind": "range", "from": 0, "to": 9}, "seed": 0,
               "timeout_s": 30, "max_frame": 100})
    );
}

#[tokio::test]
async fn an_unconfigured_client_is_unavailable_for_every_call() {
    let client = PythonClient::new(&PythonConfig::default()).unwrap();
    assert!(!client.is_configured());
    assert!(matches!(client.health().await, Err(Error::Unavailable(_))));
    assert!(matches!(client.solvers().await, Err(Error::Unavailable(_))));
}

#[test]
fn a_configured_client_reports_it() {
    let config = PythonConfig {
        url: Some("http://localhost:3160".into()),
        ..PythonConfig::default()
    };
    assert!(PythonClient::new(&config).unwrap().is_configured());
}

#[test]
fn a_request_names_its_root_only_when_it_has_one() {
    let run = RunRequest {
        project: "demo".into(),
        root: Some("/work/candidate".into()),
        path: "python/work/a.py".into(),
        entry: "go".into(),
        args: json!({}),
        seed: 0,
        timeout_s: 1,
    };
    assert_eq!(
        serde_json::to_value(&run).unwrap()["root"],
        "/work/candidate"
    );
}
