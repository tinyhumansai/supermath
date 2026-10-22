//! `file_list`: the project's files with their last results.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, MAX_LIMIT, Permission, Tool, clamp_limit,
};

use super::wire;
use crate::error::Error;
use crate::observer::FileObserver;
use crate::workspace::Workspace;

/// Lists the project's Lean and Python files, each with the last result an
/// observer recorded for it.
pub struct FileList {
    workspace: Arc<Workspace>,
    observers: Vec<Arc<dyn FileObserver>>,
}

impl fmt::Debug for FileList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileList")
            .field("workspace", &self.workspace)
            .field("observers", &self.observers.len())
            .finish()
    }
}

impl FileList {
    /// The tool over `workspace`, consulting `observers` (one per language)
    /// for last results.
    #[must_use]
    pub fn new(workspace: Arc<Workspace>, observers: Vec<Arc<dyn FileObserver>>) -> Self {
        Self {
            workspace,
            observers,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    limit: Option<u64>,
    #[serde(default)]
    cursor: Option<String>,
}

impl FileList {
    fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope, wire::Failure> {
        let args: Args = wire::parse_args(args)?;
        let prefix = args.prefix.unwrap_or_default();
        if prefix.starts_with('/')
            || prefix.contains('\\')
            || prefix.split('/').any(|segment| segment == "..")
        {
            return Err(wire::bad(
                "prefix must be relative to the project root, without ..",
            ));
        }
        let offset = args
            .cursor
            .as_deref()
            .map(|c| {
                c.parse::<usize>().map_err(|_| {
                    wire::bad("cursor must be a row offset this tool returned, such as \"50\"")
                })
            })
            .transpose()?
            .unwrap_or(0);
        let limit =
            clamp_limit(Some(args.limit.unwrap_or(MAX_LIMIT as u64))).min(ctx.budget.rows.max(1));

        let slug = ctx.project.slug();
        let matching: Vec<_> = self
            .workspace
            .list(slug)
            .map_err(|e| wire::failure(&e))?
            .into_iter()
            .filter(|p| p.as_str().starts_with(&prefix))
            .collect();
        let total = matching.len();

        let mut rows = Vec::new();
        let mut skipped = Vec::new();
        for path in matching.iter().skip(offset).take(limit) {
            let file = match self.workspace.read(slug, path) {
                Ok(file) => file,
                Err(e @ Error::Io { .. }) => return Err(wire::failure(&e)),
                // Gone, oversized, or not text since it was listed: `total`
                // still counts it, so the reply names it.
                Err(e) => {
                    skipped.push(format!("{path} was skipped: {e}"));
                    continue;
                }
            };
            let last = self
                .observers
                .iter()
                .find(|o| o.language() == path.language())
                .and_then(|o| o.last_result(slug, path));
            rows.push(json!({
                "path": path.as_str(),
                "language": path.language(),
                "sha256": file.sha256,
                "lines": file.line_count(),
                "last_result": last.as_ref().map(|l| l.result.as_str()),
                "result_sha256": last.as_ref().map(|l| l.sha256.as_str()),
                "stale": last.as_ref().map(|l| l.sha256 != file.sha256),
            }));
        }
        let envelope = skipped.into_iter().fold(
            Envelope::ok(json!({"files": rows, "total": total})),
            Envelope::with_unknown,
        );
        let next = offset + limit;
        Ok(if next < total {
            envelope.with_cursor(next.to_string())
        } else {
            envelope
        })
    }
}

#[async_trait]
impl Tool for FileList {
    fn name(&self) -> &'static str {
        "file_list"
    }

    fn description(&self) -> &'static str {
        "Given the project and optionally a directory prefix, lists its Lean and Python files \
         with their sha256, line count, and last lean_check verdict or py_run outcome. A result \
         whose result_sha256 differs from sha256 is stale. Paged, 50 rows at most."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prefix": {"type": "string", "description": "Only paths starting with this, e.g. lean/Supermath/Sendov/"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 50},
                "cursor": {"type": "string", "description": "From a previous call; continues the list"}
            },
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
        &[ErrorCode::InvalidInput, ErrorCode::BackendUnavailable]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        self.run(ctx, args).unwrap_or_else(|failure| *failure)
    }
}
