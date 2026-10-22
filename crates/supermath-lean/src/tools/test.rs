//! Unit tests for the helpers the tools share, and for the bundle.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::sync::Arc;

use supermath_files::Workspace;
use supermath_tool::{CallContext, Envelope, ErrorCode, Project};

use super::Lean;
use super::wire::{
    admit_bytes, expect_ok, guard, parse_args, parse_cursor, parse_lean_path, trim_chars,
    valid_name,
};
use crate::client::Status;
use crate::config::LeanConfig;
use crate::error::Error;

fn ctx() -> CallContext {
    CallContext::new(
        Project::new("demo", "ws:test/project:demo").unwrap(),
        "call-1",
    )
}

#[test]
fn names_are_letters_digits_and_dots_only() {
    assert!(valid_name("Nat.Prime.two_le"));
    assert!(valid_name("foo'"));
    assert!(valid_name("α₁.β"));
    for bad in [
        "",
        ".a",
        "a.",
        "a b",
        "a;b",
        "a`b",
        "a\nb",
        &"x".repeat(201),
    ] {
        assert!(!valid_name(bad), "{bad:?}");
    }
}

#[test]
fn trimming_marks_the_cut() {
    assert_eq!(trim_chars("abc", 3), "abc");
    assert_eq!(trim_chars("abcd", 3), "abc…");
}

#[test]
fn cursors_are_decimal_offsets() {
    assert_eq!(parse_cursor(None).unwrap(), 0);
    assert_eq!(parse_cursor(Some("12")).unwrap(), 12);
    assert!(parse_cursor(Some("x")).is_err());
}

#[test]
fn lean_paths_only() {
    assert!(parse_lean_path("lean/Supermath/A.lean").is_ok());
    assert!(matches!(
        parse_lean_path("python/work/a.py").unwrap_err(),
        Error::InvalidInput(_)
    ));
    assert!(parse_lean_path("../x").is_err());
}

#[test]
fn arguments_reject_unknown_fields() {
    #[derive(serde::Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct Args {
        #[allow(dead_code)]
        a: u32,
    }
    assert!(parse_args::<Args>(serde_json::json!({"a": 1})).is_ok());
    assert!(parse_args::<Args>(serde_json::json!({"a": 1, "b": 2})).is_err());
}

#[test]
fn every_status_maps_to_the_right_error() {
    assert!(expect_ok(Status::Ok, "x").is_ok());
    let cases = [
        (Status::Timeout, ErrorCode::Timeout),
        (Status::Memory, ErrorCode::Timeout),
        (Status::OutputLimit, ErrorCode::Timeout),
        (Status::NoSorry, ErrorCode::InvalidInput),
        (Status::BadPosition, ErrorCode::InvalidInput),
        (Status::BadPredicate, ErrorCode::InvalidInput),
        (Status::NotFound, ErrorCode::NotFound),
        (Status::Other, ErrorCode::BackendUnavailable),
    ];
    for (status, code) in cases {
        assert_eq!(
            expect_ok(status, "x").unwrap_err().code(),
            code,
            "{status:?}"
        );
    }
}

#[test]
fn admission_refuses_oversized_source() {
    let mut context = ctx();
    context.budget.bytes = 10;
    assert!(admit_bytes(&context, 10).is_ok());
    assert_eq!(
        admit_bytes(&context, 11).unwrap_err().code(),
        ErrorCode::BudgetExceeded
    );
}

#[tokio::test]
async fn a_cancelled_call_is_a_timeout() {
    let context = ctx();
    context.cancel.cancel();
    let envelope = guard(&context, std::future::pending::<crate::Result<Envelope>>()).await;
    assert_eq!(envelope.code(), Some(ErrorCode::Timeout));
}

#[tokio::test]
async fn a_finished_call_returns_its_envelope() {
    let envelope = guard(&ctx(), async {
        Ok(Envelope::ok(serde_json::json!({"a": 1})))
    })
    .await;
    assert!(envelope.is_ok());
    let failed = guard(&ctx(), async { Err(Error::NotFound("x".into())) }).await;
    assert_eq!(failed.code(), Some(ErrorCode::NotFound));
}

#[test]
fn the_bundle_hands_out_eight_distinct_tools() {
    let dir = tempfile::tempdir().unwrap();
    let lean = Lean::new(LeanConfig::default(), Arc::new(Workspace::new(dir.path()))).unwrap();
    let mut names: Vec<_> = lean.tools().iter().map(|t| t.name()).collect();
    assert_eq!(names.len(), 8);
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 8);
    assert!(!format!("{lean:?}").is_empty());
    assert!(!lean.client().is_configured());
}

#[tokio::test]
async fn health_without_a_service_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let lean = Lean::new(LeanConfig::default(), Arc::new(Workspace::new(dir.path()))).unwrap();
    assert!(matches!(
        lean.health().await.unwrap_err(),
        Error::Unavailable { .. }
    ));
}
