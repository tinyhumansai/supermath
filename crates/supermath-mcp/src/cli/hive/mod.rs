//! `supermath-mcp hive-tools`: the toolkit as tinyhivemind executable tools.
//!
//! An `rsi-ralph` hive package declares the programs its seats may call in
//! the `tools` array of its manifest (`tools.json` in a loop `loop new` writes; tinyhivemind's
//! `docs/specs/executable-tools.md`). This module writes one declaration per
//! registered tool, straight from the tool's own name, description, schema,
//! and permission, so the package cannot drift from the code:
//!
//! ```json
//! { "id": "lean_check", "description": "Given …",
//!   "command": ["/usr/local/bin/supermath-mcp", "call", "lean_check", "demo",
//!               "--project-dir", ".", "--env-file", "/work/supermath.env"],
//!   "args": { "type": "object", … },
//!   "stdin": "json", "output": "text", "cwd": "workspace",
//!   "side_effects": "read", "timeout_ms": 120000 }
//! ```
//!
//! Each call runs in the seat's workspace (`cwd: workspace`), and
//! `--project-dir .` makes that directory the project, so a seat editing a
//! candidate checkout reads and checks that checkout. The project slug is
//! fixed in the declaration, never taken from the call's arguments.

use serde_json::{Value, json};
use supermath_tool::{Kind, Permission, Tool, ToolRegistry};

/// How long a primitive call may run, in milliseconds. The tool's own budget
/// is shorter; this covers starting the process and composing the toolkit.
pub const PRIMITIVE_TIMEOUT_MS: u64 = 120_000;

/// How long an agentic call (a model turn behind the tool) may run.
pub const AGENTIC_TIMEOUT_MS: u64 = 600_000;

/// What every declaration's command starts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The `supermath-mcp` binary to run.
    pub binary: String,
    /// The project every call acts on.
    pub project: String,
    /// The `--env-file` every call reads, if any.
    pub env_file: Option<String>,
}

/// One declaration per tool in `registry`, in registry order.
#[must_use]
pub fn declarations(registry: &ToolRegistry, program: &Program) -> Value {
    Value::Array(
        registry
            .iter()
            .map(|tool| declaration(tool.as_ref(), program))
            .collect(),
    )
}

/// The declaration of `tool`.
fn declaration(tool: &dyn Tool, program: &Program) -> Value {
    let mut command = vec![
        program.binary.clone(),
        "call".to_owned(),
        tool.name().to_owned(),
        program.project.clone(),
        "--project-dir".to_owned(),
        ".".to_owned(),
    ];
    if let Some(env_file) = &program.env_file {
        command.extend(["--env-file".to_owned(), env_file.clone()]);
    }
    let side_effects = match tool.permission() {
        Permission::Read => "read",
        Permission::Write => "write",
    };
    let timeout_ms = match tool.kind() {
        Kind::Primitive => PRIMITIVE_TIMEOUT_MS,
        Kind::Agentic(_) => AGENTIC_TIMEOUT_MS,
    };
    json!({
        "id": tool.name(),
        "description": tool.description(),
        "command": command,
        "args": tool.schema(),
        "stdin": "json",
        "output": "text",
        "cwd": "workspace",
        "side_effects": side_effects,
        "timeout_ms": timeout_ms,
    })
}

#[cfg(test)]
mod test;
