//! The file call log: persistence across handles, and absorbed failures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use supermath_tool::{CallLog, CallRecord, Envelope, ErrorCode, Outcome};

use super::FileCallLog;

fn record(id: &str, envelope: &Envelope) -> CallRecord {
    CallRecord::new(id, "calc_factor", "demo", envelope, 3)
}

#[test]
fn a_missing_log_holds_no_calls() {
    let dir = tempfile::tempdir().unwrap();
    let log = FileCallLog::new(dir.path().join(".log/calls.jsonl"));
    assert!(log.all().is_empty());
    assert!(log.get("call-1").is_none());
}

#[test]
fn calls_recorded_by_one_handle_are_found_by_another() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".log/calls.jsonl");
    let first = FileCallLog::new(&path);
    first.record(record("a", &Envelope::ok(json!(1))));
    first.record(record("b", &Envelope::error(ErrorCode::Timeout, "slow")));

    let second = FileCallLog::new(&path);
    assert_eq!(second.path(), path);
    assert_eq!(second.get("a").unwrap().outcome, Outcome::Ok);
    assert_eq!(second.get("b").unwrap().code, Some(ErrorCode::Timeout));
    let ids: Vec<_> = second.all().into_iter().map(|r| r.call_id).collect();
    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn the_latest_record_of_a_repeated_id_wins() {
    let dir = tempfile::tempdir().unwrap();
    let log = FileCallLog::new(dir.path().join("calls.jsonl"));
    log.record(record("a", &Envelope::error(ErrorCode::Timeout, "slow")));
    log.record(record("a", &Envelope::ok(json!(1))));
    assert_eq!(log.get("a").unwrap().outcome, Outcome::Ok);
}

#[test]
fn malformed_lines_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("calls.jsonl");
    let log = FileCallLog::new(&path);
    log.record(record("a", &Envelope::ok(json!(1))));
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("{\"call_id\": \"torn\n\n");
    std::fs::write(&path, text).unwrap();
    log.record(record("b", &Envelope::ok(json!(2))));
    let ids: Vec<_> = log.all().into_iter().map(|r| r.call_id).collect();
    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn an_unwritable_log_loses_the_record_without_failing() {
    let dir = tempfile::tempdir().unwrap();
    // The log's directory is a file, so nothing can be created under it.
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "").unwrap();
    let log = FileCallLog::new(blocker.join("calls.jsonl"));
    log.record(record("a", &Envelope::ok(json!(1))));
    assert!(log.get("a").is_none());
}
