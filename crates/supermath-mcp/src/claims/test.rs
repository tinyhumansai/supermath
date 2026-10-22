//! The index remembers Lean claims, and only those, from successful records.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use supermath_lean::ClaimIndex;
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Project, Tool};

use super::{RecordedClaims, RecordsClaims};

/// A stand-in for `ledger_record` that answers one fixed envelope.
struct Canned(Envelope);

#[async_trait]
impl Tool for Canned {
    fn name(&self) -> &'static str {
        "ledger_record"
    }
    fn description(&self) -> &'static str {
        "Given a claim, records it."
    }
    fn schema(&self) -> Value {
        json!({"type": "object", "additionalProperties": false})
    }
    fn exposure(&self) -> Exposure {
        Exposure::Direct
    }
    fn permission(&self) -> Permission {
        Permission::Write
    }
    fn kind(&self) -> Kind {
        Kind::Primitive
    }
    fn errors(&self) -> &'static [ErrorCode] {
        &[ErrorCode::Forbidden]
    }
    async fn call(&self, _ctx: &CallContext, _args: Value) -> Envelope {
        self.0.clone()
    }
}

fn ctx() -> CallContext {
    CallContext::new(Project::new("demo", "ws:t/project:demo").unwrap(), "call-1")
}

fn lean_args() -> Value {
    json!({"kind": "result", "statement": "s",
        "source": {"lean": {"path": "lean/A.lean", "declaration": "A.t", "file_sha256": "x"}}})
}

fn wrapped(answer: Envelope) -> (RecordsClaims, Arc<RecordedClaims>) {
    let index = Arc::new(RecordedClaims::new());
    let inner: Arc<dyn Tool> = Arc::new(Canned(answer));
    (RecordsClaims::new(inner, index.clone()), index)
}

#[tokio::test]
async fn a_recorded_lean_claim_is_indexed_by_project_path_and_declaration() {
    let (tool, index) = wrapped(Envelope::ok(json!({"id": "claim-9"})));
    let envelope = tool.call(&ctx(), lean_args()).await;
    assert!(envelope.is_ok());
    assert_eq!(
        index.claim_id("demo", "lean/A.lean", "A.t").as_deref(),
        Some("claim-9")
    );
    assert_eq!(index.claim_id("other", "lean/A.lean", "A.t"), None);
    assert_eq!(index.claim_id("demo", "lean/A.lean", "A.u"), None);
}

#[tokio::test]
async fn a_failed_record_is_not_indexed() {
    let (tool, index) = wrapped(Envelope::error(ErrorCode::Forbidden, "no"));
    tool.call(&ctx(), lean_args()).await;
    assert_eq!(index.claim_id("demo", "lean/A.lean", "A.t"), None);
}

#[tokio::test]
async fn a_claim_with_another_source_is_not_indexed() {
    let (tool, index) = wrapped(Envelope::ok(json!({"id": "claim-1"})));
    tool.call(
        &ctx(),
        json!({"kind": "fact", "statement": "s", "source": {"supplied": "me"}}),
    )
    .await;
    assert_eq!(index.claim_id("demo", "lean/A.lean", "A.t"), None);
}

#[tokio::test]
async fn a_later_claim_on_the_same_declaration_replaces_the_earlier() {
    let index = RecordedClaims::new();
    index.insert("demo", "lean/A.lean", "A.t", "old");
    index.insert("demo", "lean/A.lean", "A.t", "new");
    assert_eq!(
        index.claim_id("demo", "lean/A.lean", "A.t").as_deref(),
        Some("new")
    );
}

#[test]
fn the_wrapper_describes_the_tool_it_wraps() {
    let (tool, _) = wrapped(Envelope::ok(json!({})));
    let inner = Canned(Envelope::ok(json!({})));
    assert_eq!(tool.name(), inner.name());
    assert_eq!(tool.description(), inner.description());
    assert_eq!(tool.schema(), inner.schema());
    assert_eq!(tool.exposure(), inner.exposure());
    assert_eq!(tool.permission(), inner.permission());
    assert_eq!(tool.kind(), inner.kind());
    assert_eq!(tool.errors(), inner.errors());
    assert!(format!("{tool:?}").contains("ledger_record"));
}
