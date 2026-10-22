//! A configurable fake [`Tool`](crate::Tool) for this crate's own tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Project, Tool};

/// A tool whose every property is a public field.
pub(crate) struct FakeTool {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) schema: Value,
    pub(crate) kind: Kind,
    pub(crate) errors: &'static [ErrorCode],
    pub(crate) reply: fn(&Value) -> Envelope,
}

/// The schema a well-behaved fake uses.
pub(crate) fn good_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"n": {"type": "integer"}},
        "required": ["n"],
        "additionalProperties": false
    })
}

fn reply(args: &Value) -> Envelope {
    if args.get("n").is_some() {
        Envelope::ok(json!({"n": args["n"]}))
    } else {
        Envelope::error(ErrorCode::InvalidInput, "n is required")
    }
}

impl FakeTool {
    /// A tool that passes the contract under the `calc` domain.
    pub(crate) fn passing() -> Self {
        Self {
            name: "calc_echo",
            description: "Given an integer, returns it. A second sentence.",
            schema: good_schema(),
            kind: Kind::Primitive,
            errors: &[ErrorCode::InvalidInput],
            reply,
        }
    }
}

#[async_trait]
impl Tool for FakeTool {
    fn name(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn schema(&self) -> Value {
        self.schema.clone()
    }
    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }
    fn permission(&self) -> Permission {
        Permission::Read
    }
    fn kind(&self) -> Kind {
        self.kind
    }
    fn errors(&self) -> &'static [ErrorCode] {
        self.errors
    }
    async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
        (self.reply)(&args)
    }
}

/// A context for tests.
pub(crate) fn ctx() -> CallContext {
    CallContext::new(
        Project::new("demo", "ws:test/project:demo").expect("valid project"),
        "call-1",
    )
}
