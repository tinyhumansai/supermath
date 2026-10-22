//! `file_read`: numbered lines of a file, or its outline.

use std::fmt::Write as _;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::wire;
use crate::outline::outline;
use crate::workspace::Workspace;

/// The most lines one call returns.
pub const MAX_READ_LINES: usize = 400;

/// Reads a Lean or Python file: a line range, or the outline.
#[derive(Debug, Clone)]
pub struct FileRead {
    workspace: Arc<Workspace>,
}

impl FileRead {
    /// The tool over `workspace`.
    #[must_use]
    pub fn new(workspace: Arc<Workspace>) -> Self {
        Self { workspace }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
    #[serde(default)]
    cursor: Option<String>,
}

impl FileRead {
    fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope, wire::Failure> {
        let args: Args = wire::parse_args(args)?;
        let path = wire::parse_path(&args.path)?;
        let file = self
            .workspace
            .read(ctx.project.slug(), &path)
            .map_err(|e| wire::failure(&e))?;
        let total = file.line_count();
        let language = path.language();

        if args.start_line.is_none() && args.end_line.is_none() && args.cursor.is_none() {
            let items = outline(language, &file.text);
            let data = json!({
                "path": path.as_str(), "language": language, "sha256": file.sha256,
                "lines": total, "outline": items,
            });
            wire::admit_bytes(ctx, data.to_string().len())?;
            return Ok(Envelope::ok(data));
        }

        let from_cursor = args
            .cursor
            .as_deref()
            .map(|c| {
                c.parse::<usize>()
                    .map_err(|_| wire::bad("cursor is not one this tool returned"))
            })
            .transpose()?;
        let start = from_cursor.or(args.start_line).unwrap_or(1);
        if start == 0 {
            return Err(wire::bad("start_line is one-based"));
        }
        if start > total.max(1) {
            return Err(wire::bad("start_line is past the end of the file"));
        }
        let requested_end = args.end_line.unwrap_or(usize::MAX);
        if requested_end < start {
            return Err(wire::bad("end_line is before start_line"));
        }
        let end = requested_end.min(total).min(start + MAX_READ_LINES - 1);
        let mut text = String::new();
        for (index, line) in file
            .text
            .lines()
            .enumerate()
            .skip(start - 1)
            .take(end.saturating_sub(start - 1))
        {
            // Writing to a String cannot fail.
            let _ = writeln!(text, "{}: {line}", index + 1);
        }
        wire::admit_bytes(ctx, text.len())?;
        let envelope = Envelope::ok(json!({
            "path": path.as_str(), "language": language, "sha256": file.sha256,
            "lines": total, "start_line": start, "end_line": end, "text": text,
        }));
        Ok(if end < requested_end.min(total) {
            envelope.with_cursor((end + 1).to_string())
        } else {
            envelope
        })
    }
}

#[async_trait]
impl Tool for FileRead {
    fn name(&self) -> &'static str {
        "file_read"
    }

    fn description(&self) -> &'static str {
        "Given a file and a line range, returns its numbered lines (at most 400 per call, with a \
         cursor for the rest) and its sha256 for a following file_edit. With no range, returns \
         the file's outline instead: each Lean declaration with its line range and whether it \
         contains a sorry, or each Python function and class with its signature."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Relative to the project root"},
                "start_line": {"type": "integer", "minimum": 1},
                "end_line": {"type": "integer", "minimum": 1},
                "cursor": {"type": "string", "description": "From a previous call; continues the read"}
            },
            "required": ["path"],
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Deferred
    }

    fn permission(&self) -> Permission {
        Permission::Read
    }

    fn kind(&self) -> Kind {
        Kind::Primitive
    }

    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::BudgetExceeded,
            ErrorCode::BackendUnavailable,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        self.run(ctx, args).unwrap_or_else(|failure| *failure)
    }
}
