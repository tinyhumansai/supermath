//! `supermath-mcp loop new`: a new rsi-ralph loop folder for one conjecture.
//!
//! A loop is a folder the `rsi-ralph` binary runs (its `docs/specs/rsi-cli.md`):
//! `rsi.json`, the `brief.md` template, a tinyhivemind package in `hive/`,
//! and the `subject/` the hive improves. This module writes one from the
//! template in `crates/supermath-mcp/template/`, compiled into the binary:
//!
//! ```text
//! <loop>/rsi.json                       the subject, hive, scorer, models, budget
//! <loop>/brief.md                       the conjecture and the epoch's task
//! <loop>/hive/                          lead, prover, refuter; the toolkit as tools
//! <loop>/subject/lean/Supermath/Statement.lean   the conjecture, frozen
//! <loop>/subject/lean/Supermath/Prove.lean       `theorem proof : Conjecture`
//! <loop>/subject/lean/Supermath/Refute.lean      `theorem refute : ¬ Conjecture`
//! ```
//!
//! The statement file must declare `Supermath.Conjecture : Prop`; it is
//! copied in verbatim. `hive.json` gets one executable tool per toolkit tool
//! (see [`declarations`]), less the `file_*` tools,
//! whose work the seats' own file tools do in their checkout, and
//! `web_search`, whose name `OpenHuman`'s own search tool has; and one toolpack per
//! domain, so each seat is given its domains by name. The scorer
//! (`supermath-mcp score proof`) and every tool run this same binary, with
//! the same `--env-file`.

use std::path::Path;

use serde_json::{Map, Value, json};
use supermath_tool::ToolRegistry;

use super::hive::{Program, declarations};
use crate::error::{Error, Result};

/// The template, as (path in the loop folder, text) pairs.
const TEMPLATE: [(&str, &str); 19] = [
    ("rsi.json", include_str!("../../../template/rsi.json")),
    ("brief.md", include_str!("../../../template/brief.md")),
    (
        "hive/hive.json",
        include_str!("../../../template/hive/hive.json"),
    ),
    (
        "hive/agents.json",
        include_str!("../../../template/hive/agents.json"),
    ),
    (
        "hive/desks.json",
        include_str!("../../../template/hive/desks.json"),
    ),
    (
        "hive/constitution.json",
        include_str!("../../../template/hive/constitution.json"),
    ),
    (
        "hive/memory.json",
        include_str!("../../../template/hive/memory.json"),
    ),
    (
        "hive/context/rules.md",
        include_str!("../../../template/hive/context/rules.md"),
    ),
    (
        "hive/roles/lead.md",
        include_str!("../../../template/hive/roles/lead.md"),
    ),
    (
        "hive/roles/prover.md",
        include_str!("../../../template/hive/roles/prover.md"),
    ),
    (
        "hive/roles/refuter.md",
        include_str!("../../../template/hive/roles/refuter.md"),
    ),
    (
        "hive/seats/lead.md",
        include_str!("../../../template/hive/seats/lead.md"),
    ),
    (
        "hive/seats/prover.md",
        include_str!("../../../template/hive/seats/prover.md"),
    ),
    (
        "hive/seats/refuter.md",
        include_str!("../../../template/hive/seats/refuter.md"),
    ),
    (
        "subject/lean/lakefile.toml",
        include_str!("../../../template/subject/lean/lakefile.toml"),
    ),
    (
        "subject/lean/lean-toolchain",
        include_str!("../../../template/subject/lean/lean-toolchain"),
    ),
    (
        "subject/lean/Supermath/Prove.lean",
        include_str!("../../../template/subject/lean/Supermath/Prove.lean"),
    ),
    (
        "subject/lean/Supermath/Refute.lean",
        include_str!("../../../template/subject/lean/Supermath/Refute.lean"),
    ),
    (
        "subject/notes/README.md",
        include_str!("../../../template/subject/notes/README.md"),
    ),
];

/// Where the conjecture goes in the loop folder.
pub const STATEMENT_PATH: &str = "subject/lean/Supermath/Statement.lean";

/// Tools a seat does not get from the toolkit. Its harness edits files in
/// its own checkout, and `OpenHuman` registers `file_read`, `file_write` and
/// `web_search` itself: a second tool of the same name stops the hive from
/// starting ("tool name is already registered").
const HARNESS_TOOLS: [&str; 5] = [
    "file_edit",
    "file_list",
    "file_read",
    "file_write",
    "web_search",
];

/// Each toolpack the template's seats name, with the tool-name prefixes (or
/// whole names) it holds.
const TOOLPACKS: [(&str, &[&str]); 5] = [
    ("lean", &["lean_"]),
    ("python", &["py_", "calc_"]),
    ("literature", &["lit_", "web_"]),
    ("ledger", &["ledger_", "recall_memory"]),
    ("ledger-read", &["ledger_claims", "recall_memory"]),
];

/// Writes a new loop folder at `dir` for the project `slug`, from the
/// template, with `statement` (Lean source declaring
/// `Supermath.Conjecture`) as its frozen statement. Every tool and the
/// scorer run `program`.
///
/// # Errors
///
/// [`Error::LoopExists`] if `dir` already exists;
/// [`Error::InvalidStatement`] if `statement` declares no `Conjecture`;
/// [`Error::Io`] if a file cannot be written.
pub fn create(
    dir: &Path,
    statement: &str,
    program: &Program,
    registry: &ToolRegistry,
) -> Result<()> {
    if dir.exists() {
        return Err(Error::LoopExists(dir.display().to_string()));
    }
    if !declares_conjecture(statement) {
        return Err(Error::InvalidStatement(
            "it must declare `Conjecture : Prop` inside `namespace Supermath`".to_owned(),
        ));
    }
    let env_file_arg = program
        .env_file
        .as_ref()
        .map_or_else(String::new, |path| format!(" --env-file {path}"));
    let render = |text: &str| {
        text.replace("{{slug}}", &program.project)
            .replace("{{binary}}", &program.binary)
            .replace("{{env_file_arg}}", &env_file_arg)
            .replace("{{statement}}", statement.trim_end())
    };
    for (path, text) in TEMPLATE {
        let text = if path == "hive/hive.json" {
            hive_manifest(&render(text), program, registry)?
        } else {
            render(text)
        };
        write(&dir.join(path), &text)?;
    }
    write(&dir.join(STATEMENT_PATH), statement)
}

/// Whether `statement` declares `Conjecture` in the `Supermath` namespace.
fn declares_conjecture(statement: &str) -> bool {
    statement.contains("namespace Supermath")
        && statement
            .lines()
            .any(|line| line.trim_start().starts_with("def Conjecture"))
}

/// The template's `hive.json` with the toolkit's tools and toolpacks added.
fn hive_manifest(text: &str, program: &Program, registry: &ToolRegistry) -> Result<String> {
    let mut manifest: Map<String, Value> =
        serde_json::from_str(text).map_err(|error| Error::Template(error.to_string()))?;
    let tools: Vec<Value> = match declarations(registry, program) {
        Value::Array(tools) => tools
            .into_iter()
            .filter(|tool| {
                !HARNESS_TOOLS
                    .iter()
                    .any(|name| tool["id"].as_str() == Some(*name))
            })
            .collect(),
        _ => Vec::new(),
    };
    let ids: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["id"].as_str())
        .collect();
    let toolpacks: Vec<Value> = TOOLPACKS
        .iter()
        .map(|(id, members)| {
            let held: Vec<&str> = ids
                .iter()
                .copied()
                .filter(|name| {
                    members
                        .iter()
                        .any(|member| name.starts_with(member) || name == member)
                })
                .collect();
            json!({"id": id, "tools": held})
        })
        .collect();
    manifest.insert("tools".to_owned(), Value::Array(tools));
    manifest.insert("toolpacks".to_owned(), Value::Array(toolpacks));
    let mut text = serde_json::to_string_pretty(&Value::Object(manifest))
        .map_err(|error| Error::Template(error.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Writes `text` to `path`, creating its directory.
fn write(path: &Path, text: &str) -> Result<()> {
    let io = |source| Error::Io {
        path: path.display().to_string(),
        source,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    std::fs::write(path, text).map_err(io)
}

#[cfg(test)]
mod test;
