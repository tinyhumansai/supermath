//! The write log: wire form, append, and read-back.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use super::{WriteLog, WriteRecord};
use crate::error::Error;

fn record(call_id: &str, new: &str) -> WriteRecord {
    WriteRecord {
        path: "lean/Supermath/A.lean".into(),
        old_sha256: None,
        new_sha256: new.into(),
        tool: "file_write".into(),
        call_id: call_id.into(),
        at_ms: 7,
    }
}

#[test]
fn record_has_the_pinned_wire_form() {
    assert_eq!(
        serde_json::to_value(record("c1", "ab")).unwrap(),
        json!({
            "path": "lean/Supermath/A.lean",
            "old_sha256": null,
            "new_sha256": "ab",
            "tool": "file_write",
            "call_id": "c1",
            "at_ms": 7
        })
    );
}

#[test]
fn missing_log_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let log = WriteLog::new(dir.path().join(".log/writes.jsonl"));
    assert_eq!(log.entries().unwrap(), []);
    assert!(log.path().ends_with("writes.jsonl"));
}

#[test]
fn append_then_read_back_in_order_and_filter_by_call() {
    let dir = tempfile::tempdir().unwrap();
    let log = WriteLog::new(dir.path().join(".log/writes.jsonl"));
    log.append(&record("c1", "a")).unwrap();
    log.append(&record("c2", "b")).unwrap();
    log.append(&record("c1", "c")).unwrap();
    let all: Vec<String> = log
        .entries()
        .unwrap()
        .into_iter()
        .map(|r| r.new_sha256)
        .collect();
    assert_eq!(all, ["a", "b", "c"]);
    let mine: Vec<String> = log
        .for_call("c1")
        .unwrap()
        .into_iter()
        .map(|r| r.new_sha256)
        .collect();
    assert_eq!(mine, ["a", "c"]);
    assert_eq!(log.for_call("nobody").unwrap(), []);
}

#[test]
fn corrupt_line_is_reported_with_its_number() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("writes.jsonl");
    let good = serde_json::to_string(&record("c1", "a")).unwrap();
    std::fs::write(&path, format!("{good}\n\nnot json\n")).unwrap();
    let error = WriteLog::new(path).entries().unwrap_err();
    assert!(
        matches!(error, Error::LogCorrupt { line: 3, .. }),
        "{error}"
    );
}

#[test]
fn unreadable_or_unwritable_log_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    // The log path is a directory: reading and appending both fail.
    let log = WriteLog::new(dir.path());
    assert!(matches!(log.entries().unwrap_err(), Error::Io { .. }));
    assert!(matches!(
        log.append(&record("c", "a")).unwrap_err(),
        Error::Io { .. }
    ));
    // The log's parent is a file: the directory cannot be created.
    let file = dir.path().join("file");
    std::fs::write(&file, "x").unwrap();
    let nested = WriteLog::new(file.join("writes.jsonl"));
    assert!(matches!(
        nested.append(&record("c", "a")).unwrap_err(),
        Error::Io { .. }
    ));
}
