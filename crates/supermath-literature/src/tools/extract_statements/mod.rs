//! `lit_extract_statements`: a paper's numbered statements and `\ref` edges.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, clamp_limit,
};

use super::{cached, coverage, parse_args, parse_cursor};
use crate::error::{Error, Result};
use crate::{ArxivClient, ArxivId, Source, extract_statements, flatten_latex};

/// Extracts statements from the LaTeX e-print without a model.
#[derive(Debug, Clone)]
pub struct LitExtractStatements {
    arxiv: Arc<ArxivClient>,
}

impl LitExtractStatements {
    /// A tool over the shared arXiv client.
    #[must_use]
    pub fn new(arxiv: Arc<ArxivClient>) -> Self {
        Self { arxiv }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    arxiv_id: String,
    limit: Option<u64>,
    cursor: Option<String>,
}

impl LitExtractStatements {
    async fn run(&self, args: Args) -> Result<Envelope> {
        let id = ArxivId::parse(&args.arxiv_id)?;
        let start = usize::try_from(parse_cursor(args.cursor.as_deref())?)
            .map_err(|_| Error::InvalidInput("cursor is out of range".into()))?;
        let limit = clamp_limit(args.limit);
        let read = self.arxiv.read(&id).await?;
        let coverage = cached(coverage("arxiv"), read.cached_age_secs);
        let locator = json!({ "arxiv_id": read.paper.arxiv_id, "version": read.paper.version });
        let Source::Latex(tree) = &read.source else {
            return Ok(Envelope::unknown(
                "the e-print has no LaTeX source (PDF only), so no statements were extracted",
            )
            .with_data(json!({ "locator": locator, "total": null, "statements": [] }))
            .with_coverage(coverage));
        };
        let flat = flatten_latex(tree);
        let extracted = extract_statements(&flat.text);
        let total = extracted.statements.len();
        let rows: Vec<Value> = extracted
            .statements
            .iter()
            .skip(start)
            .take(limit)
            .map(|s| {
                json!({
                    "label": s.label,
                    "kind": s.kind,
                    "number": s.number,
                    "section": s.section,
                    "statement": s.statement,
                    "refs": s.refs,
                })
            })
            .collect();
        let shown = start + rows.len();
        let mut envelope = Envelope::ok(json!({
            "locator": locator,
            "total": total,
            "statements": rows,
        }))
        .with_coverage(coverage);
        if shown < total {
            envelope = envelope.with_cursor(shown.to_string());
        }
        for unknown in flat.unknowns.into_iter().chain(extracted.unknowns) {
            envelope = envelope.with_unknown(unknown);
        }
        if total == 0 {
            envelope = envelope.with_unknown(
                "no statements were found; the paper may declare its theorems in a class file",
            );
        }
        Ok(envelope)
    }
}

#[async_trait]
impl Tool for LitExtractStatements {
    fn name(&self) -> &'static str {
        "lit_extract_statements"
    }

    fn description(&self) -> &'static str {
        "Given an arXiv id, returns the paper's numbered statements (theorems, \
         lemmas, definitions and the like) with label, kind, number, section, \
         LaTeX text and the labels each \\ref's, parsed from the LaTeX source \
         without a model. Statements it cannot number are listed in unknowns."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "arxiv_id": { "type": "string", "description": "such as 2310.01234 or 2310.01234v2" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
                "cursor": { "type": "string" }
            },
            "required": ["arxiv_id"],
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
            ErrorCode::RateLimited,
            ErrorCode::Timeout,
            ErrorCode::BackendUnavailable,
        ]
    }

    async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
        let result = match parse_args::<Args>(args) {
            Ok(args) => self.run(args).await,
            Err(error) => Err(error),
        };
        result.unwrap_or_else(Envelope::from)
    }
}

#[cfg(test)]
mod test;
