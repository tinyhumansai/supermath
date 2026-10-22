//! Tests for the belt: dispatch, journaling, and the `OpenHuman` adapter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use openhuman_embed::PermissionLevel;
use serde_json::json;
use supermath_tool::{CallLog as _, ErrorCode, InMemoryCallLog, Outcome};

use super::{Belt, Progress};
use crate::fake::{World, ctx};

#[derive(Default)]
struct Events(Mutex<Vec<String>>);

impl Progress for Events {
    fn tool_started(&self, parent: &str, tool: &str) {
        self.0.lock().unwrap().push(format!("{parent} -> {tool}"));
    }
    fn tool_ended(&self, parent: &str, tool: &str, outcome: Outcome) {
        self.0
            .lock()
            .unwrap()
            .push(format!("{parent} <- {tool} {outcome:?}"));
    }
}

fn belt(world: &World, progress: Option<Arc<dyn Progress>>) -> (Arc<Belt>, Arc<InMemoryCallLog>) {
    let log = Arc::new(InMemoryCallLog::new());
    let registry = world
        .registry()
        .subset(&["file_write", "file_read", "lean_check"])
        .unwrap();
    let belt = Belt::new("agent_formalize", registry, log.clone(), ctx(), progress);
    (Arc::new(belt), log)
}

#[tokio::test]
async fn a_call_is_logged_journaled_and_counted() {
    let world = World::new();
    let (belt, log) = belt(&world, None);
    let written = belt
        .call(
            "file_write",
            json!({"path": "lean/A.lean", "content": "theorem t : True := trivial"}),
        )
        .await;
    assert_eq!(written.outcome, Outcome::Ok);
    assert_eq!(belt.tool_calls(), 1);
    let notes = belt.notes();
    assert_eq!(notes[0].tool, "file_write");
    assert_eq!(notes[0].call_id, "call-1.file_write.1");
    assert_eq!(notes[0].args["path"], "lean/A.lean");
    assert_eq!(belt.succeeded("file_write").len(), 1);
    assert_eq!(belt.succeeded("file_read").len(), 0);
    assert_eq!(log.all().len(), 1);
    assert_eq!(belt.tool_names(), ["file_read", "file_write", "lean_check"]);
    assert!(format!("{belt:?}").contains("agent_formalize"));
}

#[tokio::test]
async fn a_tool_outside_the_belt_is_not_found() {
    let world = World::new();
    let (belt, _) = belt(&world, None);
    for name in ["ledger_record", "agent_formalize", "memory"] {
        let answer = belt.call(name, json!({})).await;
        assert_eq!(answer.code(), Some(ErrorCode::NotFound), "{name}");
    }
    assert_eq!(belt.succeeded("ledger_record").len(), 0);
}

#[tokio::test]
async fn progress_sees_each_call_start_and_end() {
    let world = World::new();
    let events = Arc::new(Events::default());
    let (belt, _) = belt(&world, Some(events.clone()));
    belt.call("file_read", json!({"path": "lean/Missing.lean"}))
        .await;
    assert_eq!(
        *events.0.lock().unwrap(),
        [
            "agent_formalize -> file_read",
            "agent_formalize <- file_read Error"
        ]
    );
}

#[tokio::test]
async fn the_journal_keeps_only_the_newest_entries() {
    let world = World::new();
    let (belt, _) = belt(&world, None);
    for _ in 0..2005 {
        belt.call("lean_check", json!({"path": "lean/Missing.lean"}))
            .await;
    }
    let notes = belt.notes();
    assert_eq!(notes.len(), 2000);
    assert_eq!(notes.last().unwrap().call_id, "call-1.lean_check.2005");
    assert_eq!(belt.tool_calls(), 2005);
}

#[tokio::test]
async fn host_tools_expose_each_primitive_by_name_with_the_right_permission() {
    let world = World::new();
    let (belt, _) = belt(&world, None);
    let tools = belt.host_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert_eq!(names, ["file_read", "file_write", "lean_check"]);
    let by_name = |name: &str| tools.iter().find(|t| t.name() == name).unwrap();
    assert_eq!(
        by_name("file_write").permission_level(),
        PermissionLevel::Write
    );
    assert!(by_name("file_write").policy().side_effects.writes_files);
    assert_eq!(
        by_name("file_read").permission_level(),
        PermissionLevel::ReadOnly
    );
    assert!(by_name("file_read").policy().side_effects.read_only);
    assert_ne!(by_name("lean_check").description().len(), 0);
    assert_eq!(by_name("lean_check").parameters_schema()["type"], "object");
    assert_eq!(
        by_name("lean_check").exposure(),
        openhuman_embed::ToolExposure::Direct
    );
    let direct = super::HostTool::new(
        Arc::clone(&belt),
        Arc::clone(belt.registry.get("lean_check").unwrap()),
    );
    assert_eq!(format!("{direct:?}"), "HostTool(\"lean_check\")");
}

#[tokio::test]
async fn a_host_tool_runs_through_the_belt_and_returns_the_envelope_as_json() {
    let world = World::new();
    let (belt, _) = belt(&world, None);
    let tools = belt.host_tools();
    let write = tools.iter().find(|t| t.name() == "file_write").unwrap();
    let ok = write
        .execute(json!({"path": "lean/A.lean", "content": "theorem t : True := trivial"}))
        .await
        .unwrap();
    assert!(!ok.is_error);
    assert!(ok.output().contains("\"outcome\":\"ok\""));
    let read = tools.iter().find(|t| t.name() == "file_read").unwrap();
    let refused = read
        .execute(json!({"path": "lean/Nope.lean"}))
        .await
        .unwrap();
    assert!(!refused.is_error, "a refusal is an answer, not a harness failure");
    assert!(refused.output().contains("NOT_FOUND"));
    assert_eq!(belt.tool_calls(), 2);
}

#[derive(Default)]
struct Seen(Mutex<Vec<(String, String)>>);

impl super::Watcher for Seen {
    fn observed(&self, note: &super::CallNote) {
        self.0
            .lock()
            .unwrap()
            .push((note.tool.clone(), note.call_id.clone()));
    }
}

#[tokio::test]
async fn a_watcher_sees_every_call_once_in_order_with_its_call_id() {
    let world = World::new();
    let registry = world
        .registry()
        .subset(&["file_write", "file_read"])
        .unwrap();
    let seen = Arc::new(Seen::default());
    let belt = Belt::new(
        "agent_settle",
        registry,
        Arc::new(InMemoryCallLog::new()),
        ctx(),
        None,
    )
    .with_watcher(seen.clone());
    belt.call(
        "file_write",
        json!({"path": "lean/Supermath/A.lean", "content": "x"}),
    )
    .await;
    belt.call("file_read", json!({"path": "lean/Supermath/A.lean"}))
        .await;
    belt.call("lean_check", json!({"path": "lean/Supermath/A.lean"}))
        .await;
    assert_eq!(
        *seen.0.lock().unwrap(),
        [
            ("file_write".to_owned(), "call-1.file_write.1".to_owned()),
            ("file_read".to_owned(), "call-1.file_read.2".to_owned()),
            ("lean_check".to_owned(), "call-1.lean_check.3".to_owned()),
        ]
    );
}
