//! `lean_search`: Mathlib declarations by name, type pattern, or meaning.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, clamp_limit,
};

use super::Shared;
use super::wire::{coverage, guard, parse_args, trim_chars};
use crate::client::{SearchRequest, SearchRow};
use crate::error::{Error, Result};
use crate::hosted::Index;

/// Longest type returned per row, in characters.
const MAX_TYPE_CHARS: usize = 300;
/// Longest query accepted, in characters.
const MAX_QUERY_CHARS: usize = 300;

/// Searches Mathlib.
#[derive(Debug)]
pub struct LeanSearch {
    shared: Arc<Shared>,
}

impl LeanSearch {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

/// How `lean_search` interprets its query.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Name,
    Pattern,
    Natural,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    query: String,
    mode: Mode,
    #[serde(default)]
    limit: Option<u64>,
}

impl LeanSearch {
    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let query = args.query.trim();
        if query.is_empty() || query.chars().count() > MAX_QUERY_CHARS {
            return Err(Error::InvalidInput(format!(
                "query must be 1 to {MAX_QUERY_CHARS} characters"
            )));
        }
        let limit = clamp_limit(args.limit).min(ctx.budget.rows.max(1));
        let (rows, coverage, unknown) = match args.mode {
            Mode::Name => {
                let reply = self
                    .shared
                    .client
                    .search(&SearchRequest {
                        query: query.to_owned(),
                        limit,
                        timeout_ms: self.shared.config.query_timeout_ms,
                    })
                    .await?;
                super::wire::expect_ok(reply.status, "the name search")?;
                let searched = format!("mathlib {} by name", reply.mathlib);
                (
                    reply.results,
                    coverage(
                        searched,
                        &["the project's own declarations (use lean_decl)"],
                    ),
                    None,
                )
            }
            Mode::Pattern | Mode::Natural => {
                let index = if matches!(args.mode, Mode::Pattern) {
                    Index::Loogle
                } else {
                    Index::LeanSearch
                };
                let rows = self.shared.hosted.search(index, query, limit).await?;
                (
                    rows,
                    coverage(
                        index.name(),
                        &["the project's own declarations (use lean_decl)"],
                    ),
                    Some(format!(
                        "{} indexes its own Mathlib revision, which may differ from the pinned one",
                        index.name()
                    )),
                )
            }
        };
        let rows: Vec<Value> = rows
            .into_iter()
            .take(limit)
            .map(|SearchRow { name, ty, module }| {
                json!({"name": name, "type": trim_chars(&ty, MAX_TYPE_CHARS), "module": module})
            })
            .collect();
        let mut envelope = Envelope::ok(json!({"results": rows})).with_coverage(coverage);
        if let Some(note) = unknown {
            envelope = envelope.with_unknown(note);
        }
        Ok(envelope)
    }
}

#[async_trait]
impl Tool for LeanSearch {
    fn name(&self) -> &'static str {
        "lean_search"
    }

    fn description(&self) -> &'static str {
        "Given a query and a mode, finds Mathlib declarations. Mode name matches a name fragment \
         in the pinned Mathlib (Nat.Prime.two_le); pattern asks Loogle for a type pattern \
         ((?a + ?b) ^ 2); natural asks LeanSearch in words (sum of two squares). Each row is \
         name, type (trimmed to 300 characters), and module; coverage names the index, and a \
         hosted index on another Mathlib revision is noted in unknowns."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "minLength": 1, "maxLength": 300},
                "mode": {"type": "string", "enum": ["name", "pattern", "natural"]},
                "limit": {"type": "integer", "minimum": 1, "maximum": 50}
            },
            "required": ["query", "mode"],
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
            ErrorCode::BackendUnavailable,
            ErrorCode::RateLimited,
            ErrorCode::Timeout,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        guard(ctx, self.run(ctx, args)).await
    }
}
