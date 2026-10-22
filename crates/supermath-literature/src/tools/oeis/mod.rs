//! `lit_oeis`: integer sequences by terms or by words.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool, clamp_limit,
};

use super::{cached, parse_args, parse_cursor};
use crate::OeisClient;
use crate::error::{Error, Result};
use supermath_tool::Coverage;

/// The fewest terms OEIS can usefully match.
const MIN_TERMS: usize = 4;
/// The most terms accepted in one query.
const MAX_QUERY_TERMS: usize = 60;

/// Searches the OEIS.
#[derive(Debug, Clone)]
pub struct LitOeis {
    oeis: Arc<OeisClient>,
}

impl LitOeis {
    /// A tool over the shared OEIS client.
    #[must_use]
    pub fn new(oeis: Arc<OeisClient>) -> Self {
        Self { oeis }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    terms: Option<Vec<i64>>,
    query: Option<String>,
    limit: Option<u64>,
    cursor: Option<String>,
}

impl LitOeis {
    async fn run(&self, args: Args) -> Result<Envelope> {
        let (q, by_terms) = match (&args.terms, &args.query) {
            (Some(terms), None) => {
                if !(MIN_TERMS..=MAX_QUERY_TERMS).contains(&terms.len()) {
                    return Err(Error::InvalidInput(format!(
                        "give between {MIN_TERMS} and {MAX_QUERY_TERMS} terms"
                    )));
                }
                let joined: Vec<String> = terms.iter().map(ToString::to_string).collect();
                (joined.join(","), true)
            }
            (None, Some(query)) if !query.trim().is_empty() => (query.trim().to_owned(), false),
            _ => {
                return Err(Error::InvalidInput(
                    "give either terms or a non-empty query, not both".into(),
                ));
            }
        };
        let start = parse_cursor(args.cursor.as_deref())?;
        let page = self.oeis.search(&q, start, clamp_limit(args.limit)).await?;
        let mut envelope = Envelope::ok(json!({
            "total": page.total,
            "sequences": page.sequences,
        }))
        .with_coverage(cached(
            Coverage::new(["oeis"], ["sequences not in the oeis"]),
            page.cached_age_secs,
        ));
        if let Some(next) = page.next {
            envelope = envelope.with_cursor(next.to_string());
        }
        if by_terms {
            envelope = envelope.with_unknown(
                "the oeis matches a subsequence of terms, so a match is a lead, not an identification",
            );
        }
        if page.sequences.is_empty() {
            envelope = envelope.with_unknown(
                "no oeis sequence matched; that says nothing about sequences the oeis does not hold",
            );
        }
        Ok(envelope)
    }
}

#[async_trait]
impl Tool for LitOeis {
    fn name(&self) -> &'static str {
        "lit_oeis"
    }

    fn description(&self) -> &'static str {
        "Given at least four integer terms or a text query, returns matching \
         OEIS sequences with id, name, up to 30 terms, up to five formula \
         lines and references. The OEIS matches subsequences, so a match is \
         a lead, not an identification."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "terms": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "minItems": MIN_TERMS,
                    "maxItems": MAX_QUERY_TERMS
                },
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
                "cursor": { "type": "string" }
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
        &[
            ErrorCode::InvalidInput,
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
