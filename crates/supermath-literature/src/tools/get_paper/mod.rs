//! `lit_get_paper`: a paper's metadata and section index, or one section.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::{cached, coverage, parse_args, parse_cursor};
use crate::error::{Error, Result};
use crate::{ArxivClient, ArxivId, Read, Source, flatten_latex, outline};

/// The most characters of a section one call returns.
const SECTION_PAGE_CHARS: usize = 12_000;

/// Reads a paper from its LaTeX e-print.
#[derive(Debug, Clone)]
pub struct LitGetPaper {
    arxiv: Arc<ArxivClient>,
}

impl LitGetPaper {
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
    section: Option<String>,
    cursor: Option<String>,
}

const PDF_ONLY: &str = "the e-print has no LaTeX source (PDF only), so no text or section index \
                        was read; formulas from PDF text would be unreliable";

fn metadata(read: &Read) -> Value {
    let paper = &read.paper;
    json!({
        "source": "arxiv",
        "arxiv_id": paper.arxiv_id,
        "version": paper.version,
        "title": paper.title,
        "authors": paper.authors,
        "primary_category": paper.primary_category,
        "categories": paper.categories,
        "published": paper.published,
        "updated": paper.updated,
        "abstract": paper.summary,
    })
}

impl LitGetPaper {
    async fn run(&self, args: Args) -> Result<Envelope> {
        let id = ArxivId::parse(&args.arxiv_id)?;
        if args.section.is_none() && args.cursor.is_some() {
            return Err(Error::InvalidInput("cursor needs a section".into()));
        }
        let offset = usize::try_from(parse_cursor(args.cursor.as_deref())?).unwrap_or(usize::MAX);
        let read = self.arxiv.read(&id).await?;
        let mut data = metadata(&read);
        let coverage = cached(coverage("arxiv"), read.cached_age_secs);
        let Source::Latex(tree) = &read.source else {
            return Ok(match args.section {
                None => {
                    data["source"] = json!("pdf");
                    data["sections"] = json!([]);
                    Envelope::ok(data).with_unknown(PDF_ONLY)
                }
                Some(_) => Envelope::unknown(PDF_ONLY).with_data(data),
            }
            .with_coverage(coverage));
        };
        let flat = flatten_latex(tree);
        let outline = outline(&flat.text);
        let mut envelope = if let Some(wanted) = &args.section {
            let index = outline.find(wanted).ok_or_else(|| {
                Error::NotFound(format!("{} has no section {wanted}", read.paper.arxiv_id))
            })?;
            let text = outline.section_text(&flat.text, index);
            let total = text.chars().count();
            let start = text
                .char_indices()
                .nth(offset)
                .map_or(text.len(), |(b, _)| b);
            let end = text[start..]
                .char_indices()
                .nth(SECTION_PAGE_CHARS)
                .map_or(text.len(), |(b, _)| start + b);
            let heading = &outline.headings[index];
            data["section"] = json!({ "number": heading.number, "title": heading.title });
            data["text"] = json!(&text[start..end]);
            data["chars_total"] = json!(total);
            data["offset"] = json!(offset.min(total));
            let more = end < text.len();
            let envelope = Envelope::ok(data);
            if more {
                envelope
                    .with_cursor((offset.min(total) + text[start..end].chars().count()).to_string())
            } else {
                envelope
            }
        } else {
            data["source"] = json!("latex");
            data["sections"] = json!(
                outline
                    .headings
                    .iter()
                    .enumerate()
                    .map(|(i, h)| json!({
                        "number": h.number,
                        "title": h.title,
                        "chars": outline.section_text(&flat.text, i).chars().count(),
                    }))
                    .collect::<Vec<_>>()
            );
            let next = outline
                .headings
                .iter()
                .find_map(|h| h.number.clone())
                .map(|n| {
                    format!(
                        "lit_get_paper {{\"arxiv_id\": \"{}\", \"section\": \"{n}\"}}",
                        read.paper.arxiv_id
                    )
                });
            let envelope = Envelope::ok(data);
            match next {
                Some(call) => envelope.with_next([
                    call,
                    format!(
                        "lit_extract_statements {{\"arxiv_id\": \"{}\"}}",
                        read.paper.arxiv_id
                    ),
                ]),
                None => envelope,
            }
        };
        for unknown in flat.unknowns {
            envelope = envelope.with_unknown(unknown);
        }
        Ok(envelope.with_coverage(coverage))
    }
}

#[async_trait]
impl Tool for LitGetPaper {
    fn name(&self) -> &'static str {
        "lit_get_paper"
    }

    fn description(&self) -> &'static str {
        "Given an arXiv id, returns the paper's metadata, abstract and section \
         index, or with a section number the text of that section (at most \
         12,000 characters, with a cursor for the rest). Read from the LaTeX \
         e-print, flattened; a PDF-only paper is flagged in unknowns."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "arxiv_id": { "type": "string", "description": "such as 2310.01234 or 2310.01234v2" },
                "section": { "type": "string", "description": "a section number such as 3.2" },
                "cursor": { "type": "string", "description": "from a previous call on the same section" }
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
