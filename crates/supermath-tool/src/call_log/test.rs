//! Tests for the in-memory call log.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use serde_json::json;

use super::{CallLog, CallRecord, InMemoryCallLog};
use crate::{Envelope, ErrorCode, Outcome};

fn record(id: &str, envelope: &Envelope) -> CallRecord {
    CallRecord::new(id, "calc_factor", "p", envelope, 3)
}

#[test]
fn records_are_found_by_call_id_in_order() {
    let log = InMemoryCallLog::new();
    log.record(record("a", &Envelope::ok(json!(1))));
    log.record(record("b", &Envelope::error(ErrorCode::Timeout, "slow")));
    assert_eq!(log.get("b").unwrap().code, Some(ErrorCode::Timeout));
    assert_eq!(log.get("a").unwrap().outcome, Outcome::Ok);
    assert!(log.get("missing").is_none());
    let ids: Vec<_> = log.all().into_iter().map(|r| r.call_id).collect();
    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn a_record_has_the_pinned_wire_form() {
    let r = record("a", &Envelope::error(ErrorCode::NotFound, "x"));
    assert_eq!(
        serde_json::to_value(&r).unwrap(),
        json!({"call_id": "a", "tool": "calc_factor", "project": "p",
               "outcome": "error", "code": "NOT_FOUND", "elapsed_ms": 3})
    );
}

#[test]
fn survives_a_poisoned_lock() {
    let log = Arc::new(InMemoryCallLog::new());
    log.record(record("a", &Envelope::ok(json!(1))));
    let clone = Arc::clone(&log);
    let _ = std::thread::spawn(move || {
        let _guard = clone.records.lock().unwrap();
        panic!("poison the lock");
    })
    .join();
    log.record(record("b", &Envelope::ok(json!(2))));
    assert_eq!(log.all().len(), 2);
}
