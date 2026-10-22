//! Tests for the registry and `dispatch`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use serde_json::json;

use super::ToolRegistry;
use crate::fake::{FakeTool, ctx};
use crate::{CallLog, Error, ErrorCode, InMemoryCallLog, Outcome};

fn registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(FakeTool::passing())).unwrap();
    let other = FakeTool {
        name: "calc_other",
        ..FakeTool::passing()
    };
    registry.register(Arc::new(other)).unwrap();
    registry
}

#[test]
fn registers_and_looks_up_tools_in_name_order() {
    let registry = registry();
    assert_eq!(registry.len(), 2);
    assert!(!registry.is_empty());
    assert_eq!(
        registry.names().collect::<Vec<_>>(),
        ["calc_echo", "calc_other"]
    );
    assert_eq!(registry.iter().count(), 2);
    assert!(registry.get("calc_echo").is_some());
    assert!(registry.get("nope").is_none());
    assert_eq!(format!("{registry:?}"), r#"["calc_echo", "calc_other"]"#);
}

#[test]
fn refuses_a_duplicate_name() {
    let mut registry = registry();
    let error = registry
        .register(Arc::new(FakeTool::passing()))
        .unwrap_err();
    assert_eq!(error, Error::DuplicateTool("calc_echo".into()));
}

#[test]
fn extend_merges_and_reports_collisions() {
    let mut merged = ToolRegistry::new();
    merged.extend(&registry()).unwrap();
    assert_eq!(merged.len(), 2);
    assert!(matches!(
        merged.extend(&registry()),
        Err(Error::DuplicateTool(_))
    ));
}

#[test]
fn subset_keeps_only_named_tools() {
    let subset = registry().subset(&["calc_other", "calc_other"]).unwrap();
    assert_eq!(subset.names().collect::<Vec<_>>(), ["calc_other"]);
    assert_eq!(
        registry().subset(&["ghost"]).unwrap_err(),
        Error::UnknownTool("ghost".into())
    );
}

#[tokio::test]
async fn dispatch_calls_the_tool_and_logs_the_call() {
    let log = InMemoryCallLog::new();
    let envelope = registry()
        .dispatch(&log, "calc_echo", &ctx(), json!({"n": 2}))
        .await;
    assert_eq!(envelope.outcome, Outcome::Ok);
    let record = log.get("call-1").unwrap();
    assert_eq!(record.tool, "calc_echo");
    assert_eq!(record.project, "demo");
}

#[tokio::test]
async fn dispatch_of_an_unknown_tool_is_not_found_and_logged() {
    let log = InMemoryCallLog::new();
    let envelope = registry().dispatch(&log, "ghost", &ctx(), json!({})).await;
    assert_eq!(envelope.code(), Some(ErrorCode::NotFound));
    assert_eq!(log.all()[0].code, Some(ErrorCode::NotFound));
}

#[test]
fn registered_tools_expose_their_descriptors() {
    use crate::{Exposure, Permission};
    let registry = registry();
    let tool = registry.get("calc_echo").unwrap();
    assert_eq!(tool.exposure(), Exposure::Deferred);
    assert_eq!(tool.permission(), Permission::Read);
}
