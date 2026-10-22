//! `lit_citations`: what a paper cites, and what cites it.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, clamp_limit,
};

use super::{cached, coverage, parse_args, parse_cursor};
use crate::error::Result;
use crate::{ArxivId, Direction, SemanticScholarClient};

/// Looks up references and citations on Semantic Scholar.
#[derive(Debug, Clone)]
pub struct LitCitations {
    scholar: Arc<SemanticScholarClient>,
}

impl LitCitations {
    /// A tool over the shared Semantic Scholar client.
    #[must_use]
    pub fn new(scholar: Arc<SemanticScholarClient>) -> Self {
        Self { scholar }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    arxiv_id: String,
    direction: Dir,
    limit: Option<u64>,
    cursor: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Dir {
    References,
    CitedBy,
}

impl LitCitations {
    async fn run(&self, args: Args) -> Result<Envelope> {
        let id = ArxivId::parse(&args.arxiv_id)?;
        let offset = parse_cursor(args.cursor.as_deref())?;
        let direction = match args.direction {
            Dir::References => Direction::References,
            Dir::CitedBy => Direction::CitedBy,
        };
        let page = self
            .scholar
            .citations(&id, direction, offset, clamp_limit(args.limit))
            .await?;
        let mut envelope = Envelope::ok(json!({
            "arxiv_id": id.base(),
            "direction": match args.direction { Dir::References => "references", Dir::CitedBy => "cited_by" },
            "papers": page.papers,
        }))
        .with_coverage(cached(coverage("semantic_scholar"), page.cached_age_secs));
        if let Some(next) = page.next {
            envelope = envelope.with_cursor(next.to_string());
        }
        if let Some(found) = page.papers.iter().find_map(|p| p.arxiv_id.clone()) {
            envelope = envelope.with_next([format!("lit_get_paper {{\"arxiv_id\": \"{found}\"}}")]);
        }
        Ok(envelope.with_unknown(
            "semantic scholar's citation graph is incomplete; an absent paper may still cite or be cited",
        ))
    }
}

#[async_trait]
impl Tool for LitCitations {
    fn name(&self) -> &'static str {
        "lit_citations"
    }

    fn description(&self) -> &'static str {
        "Given an arXiv id and a direction (references or cited_by), returns \
         the papers it cites or the papers citing it, from Semantic Scholar, \
         each with its arXiv id when it has one. Use cited_by to ask whether \
         anyone has built on or refuted a result."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "arxiv_id": { "type": "string", "description": "such as 2310.01234" },
                "direction": { "type": "string", "enum": ["references", "cited_by"] },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
                "cursor": { "type": "string" }
            },
            "required": ["arxiv_id", "direction"],
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
