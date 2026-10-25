//! A tool's wire description and an envelope's wire result.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use async_trait::async_trait;
use serde_json::{Value, json};
use supermath_calc::CalcFactor;
use supermath_tool::{Backend, CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::{META_KEY, describe, result};

fn first_text(result: &rmcp::model::CallToolResult) -> Value {
    serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
}

#[test]
fn a_read_tool_is_described_read_only_with_its_exposure_and_kind() {
    let tool = describe(&CalcFactor);
    assert_eq!(tool.name, "calc_factor");
    assert_eq!(tool.annotations.unwrap().read_only_hint, Some(true));
    let meta = tool.meta.unwrap().0;
    assert_eq!(meta[META_KEY]["kind"], "primitive");
    assert_eq!(
        meta[META_KEY]["exposure"],
        serde_json::to_value(CalcFactor.exposure()).unwrap()
    );
    assert_eq!(tool.input_schema.get("type"), Some(&json!("object")));
}

#[test]
fn an_ok_envelope_is_text_json_with_the_call_id_and_no_error_flag() {
    let envelope = Envelope::ok(json!({"x": 1}));
    let out = result("call-7", &envelope);
    assert_eq!(out.is_error, Some(false));
    let body = first_text(&out);
    assert_eq!(body["outcome"], "ok");
    assert_eq!(body["call_id"], "call-7");
    assert_eq!(body["data"]["x"], 1);
}

#[test]
fn an_error_envelope_sets_is_error() {
    let out = result("call-1", &Envelope::error(ErrorCode::NotFound, "gone"));
    assert_eq!(out.is_error, Some(true));
    assert_eq!(first_text(&out)["error"]["code"], "NOT_FOUND");
}

#[test]
fn an_unknown_envelope_is_an_answer_not_an_error() {
    let out = result("call-1", &Envelope::unknown("ran out of time"));
    assert_eq!(out.is_error, Some(false));
    assert_eq!(first_text(&out)["outcome"], "unknown");
}

/// A tool with a chosen permission and kind.
struct Odd(Permission, Kind, Value);

#[async_trait]
impl Tool for Odd {
    fn name(&self) -> &'static str {
        "odd_tool"
    }
    fn description(&self) -> &'static str {
        "Given nothing, returns nothing."
    }
    fn schema(&self) -> Value {
        self.2.clone()
    }
    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }
    fn permission(&self) -> Permission {
        self.0
    }
    fn kind(&self) -> Kind {
        self.1
    }
    fn errors(&self) -> &'static [ErrorCode] {
        &[]
    }
    async fn call(&self, _ctx: &CallContext, _args: Value) -> Envelope {
        Envelope::ok(json!({}))
    }
}

#[test]
fn a_write_tool_is_not_read_only() {
    let tool = describe(&Odd(
        Permission::Write,
        Kind::Primitive,
        json!({"type": "object"}),
    ));
    assert_eq!(tool.annotations.unwrap().read_only_hint, Some(false));
    assert_eq!(tool.meta.unwrap().0[META_KEY]["exposure"], "deferred");
}

#[test]
fn an_agentic_kind_names_its_backend() {
    let gemini = describe(&Odd(
        Permission::Read,
        Kind::Agentic(Backend::Gemini),
        json!({"type": "object"}),
    ));
    assert_eq!(gemini.meta.unwrap().0[META_KEY]["kind"], "agentic:gemini");
}

#[test]
fn a_schema_that_is_not_an_object_becomes_an_empty_one() {
    let tool = describe(&Odd(Permission::Read, Kind::Primitive, json!("nope")));
    assert!(tool.input_schema.is_empty());
}
