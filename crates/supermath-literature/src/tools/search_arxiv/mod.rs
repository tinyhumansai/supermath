//! `lit_search_arxiv`: papers matching a query.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, clamp_limit,
};

use super::{cached, coverage, parse_args, parse_cursor, trim_chars};
use crate::{ArxivClient, Paper, SearchQuery, SortOrder};

/// How many characters of abstract a row carries.
const ABSTRACT_CHARS: usize = 600;
/// How many authors a row names before it counts the rest.
const AUTHORS_SHOWN: usize = 3;

/// Searches arXiv; a direct tool, always in the model's catalogue.
#[derive(Debug, Clone)]
pub struct LitSearchArxiv {
    arxiv: Arc<ArxivClient>,
}

impl LitSearchArxiv {
    /// A tool over the shared arXiv client.
    #[must_use]
    pub fn new(arxiv: Arc<ArxivClient>) -> Self {
        Self { arxiv }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    query: Option<String>,
    title: Option<String>,
    author: Option<String>,
    category: Option<String>,
    since: Option<String>,
    sort: Option<Sort>,
    limit: Option<u64>,
    cursor: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Sort {
    Relevance,
    Recent,
}

fn row(paper: &Paper) -> Value {
    json!({
        "source": "arxiv",
        "arxiv_id": paper.arxiv_id,
        "version": paper.version,
        "title": paper.title,
        "authors": paper.authors.iter().take(AUTHORS_SHOWN).collect::<Vec<_>>(),
        "author_count": paper.authors.len(),
        "primary_category": paper.primary_category,
        "published": paper.published,
        "abstract": trim_chars(&paper.summary, ABSTRACT_CHARS),
    })
}

#[async_trait]
impl Tool for LitSearchArxiv {
    fn name(&self) -> &'static str {
        "lit_search_arxiv"
    }

    fn description(&self) -> &'static str {
        "Given a query (and optionally a title, author, category, or earliest \
         date), returns arXiv papers with their ids, authors, categories and \
         trimmed abstracts. Searches arXiv only; zbMATH, MathSciNet, journals \
         outside arXiv and books are not covered."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "free text, matched against every field" },
                "title": { "type": "string" },
                "author": { "type": "string" },
                "category": { "type": "string", "description": "an arXiv category such as math.NT" },
                "since": { "type": "string", "description": "YYYY-MM-DD" },
                "sort": { "type": "string", "enum": ["relevance", "recent"] },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
                "cursor": { "type": "string" }
            },
            "additionalProperties": false
        })
    }

    fn exposure(&self) -> Exposure {
        Exposure::Direct
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
            ErrorCode::RateLimited,
            ErrorCode::Timeout,
            ErrorCode::BackendUnavailable,
        ]
    }

    async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
        let run = async {
            let args: Args = parse_args(args)?;
            let limit = clamp_limit(args.limit);
            let query = SearchQuery {
                text: args.query,
                title: args.title,
                author: args.author,
                category: args.category,
                since: args.since,
                sort: match args.sort {
                    Some(Sort::Recent) => SortOrder::Recent,
                    Some(Sort::Relevance) | None => SortOrder::Relevance,
                },
                start: parse_cursor(args.cursor.as_deref())?,
                limit,
            };
            self.arxiv.search(&query).await
        };
        let page = match run.await {
            Ok(page) => page,
            Err(error) => return error.into(),
        };
        let rows: Vec<Value> = page.papers.iter().map(row).collect();
        let shown = page.start + rows.len() as u64;
        let mut envelope = Envelope::ok(json!({ "total": page.total, "papers": rows }))
            .with_coverage(cached(coverage("arxiv"), page.cached_age_secs));
        if let Some(top) = page.papers.first() {
            envelope = envelope.with_next([format!(
                "lit_get_paper {{\"arxiv_id\": \"{}\"}}",
                top.arxiv_id
            )]);
        } else {
            envelope = envelope.with_unknown(
                "no arxiv paper matched; this says nothing about zbmath, mathscinet, journals or books",
            );
        }
        if shown < page.total && !page.papers.is_empty() {
            envelope = envelope.with_cursor(shown.to_string());
        }
        envelope
    }
}

#[cfg(test)]
mod test;
