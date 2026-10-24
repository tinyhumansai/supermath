//! Executable-tool declarations, generated from the registry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Value, json};

use super::{AGENTIC_TIMEOUT_MS, PRIMITIVE_TIMEOUT_MS, Program, declarations};
use crate::{Config, Toolkit};

fn program(env_file: Option<&str>) -> Program {
    Program {
        binary: "/bin/supermath-mcp".into(),
        project: "demo".into(),
        env_file: env_file.map(str::to_owned),
    }
}

fn generated(env_file: Option<&str>) -> Vec<Value> {
    let toolkit = Toolkit::compose(&Config::from_lookup(|_| None).unwrap(), "demo").unwrap();
    let Value::Array(tools) = declarations(&toolkit.registry(), &program(env_file)) else {
        panic!("declarations are an array");
    };
    tools
}

fn find<'a>(tools: &'a [Value], id: &str) -> &'a Value {
    tools.iter().find(|tool| tool["id"] == id).unwrap()
}

#[test]
fn declares_every_registered_tool() {
    let toolkit = Toolkit::compose(&Config::from_lookup(|_| None).unwrap(), "demo").unwrap();
    let tools = generated(None);
    assert_eq!(tools.len(), toolkit.registry().iter().count());
    for tool in toolkit.registry().iter() {
        let declared = find(&tools, tool.name());
        assert_eq!(declared["description"], tool.description());
        assert_eq!(declared["args"], tool.schema());
    }
}

#[test]
fn a_declaration_has_the_pinned_wire_form() {
    let tools = generated(Some("/work/supermath.env"));
    let factor = find(&tools, "calc_factor");
    let mut expected = factor.clone();
    let object = expected.as_object_mut().unwrap();
    object.remove("description");
    object.remove("args");
    assert_eq!(
        expected,
        json!({
            "id": "calc_factor",
            "command": ["/bin/supermath-mcp", "call", "calc_factor", "demo",
                        "--workspace", ".", "--env-file", "/work/supermath.env"],
            "stdin": "json",
            "output": "text",
            "cwd": "workspace",
            "side_effects": "read",
            "timeout_ms": PRIMITIVE_TIMEOUT_MS,
        })
    );
}

#[test]
fn without_an_env_file_the_command_ends_at_the_workspace() {
    let tools = generated(None);
    assert_eq!(
        find(&tools, "calc_factor")["command"],
        json!([
            "/bin/supermath-mcp",
            "call",
            "calc_factor",
            "demo",
            "--workspace",
            "."
        ])
    );
}

#[test]
fn writes_and_agentic_tools_are_marked_as_such() {
    let tools = generated(None);
    assert_eq!(find(&tools, "file_write")["side_effects"], "write");
    assert_eq!(
        find(&tools, "web_research")["timeout_ms"],
        AGENTIC_TIMEOUT_MS
    );
}
