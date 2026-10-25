//! `lean_search_counterexample`: exhaustive search of a decidable predicate
//! over a finite range, run by the Lean service.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_files::{ProjectPath, WriteMeta, sha256_hex};
use supermath_tool::{
    CallContext, Coverage, Envelope, ErrorCode, Exposure, Kind, Permission, Tool,
};

use super::Shared;
use super::wire::{coverage, expect_ok, guard, parse_args, parse_lean_path, valid_name};
use crate::client::{CounterRequest, Status};
use crate::declarations::declarations;
use crate::error::{Error, Result};
use crate::verdict::Verdict;

/// Searches a range for a witness and records one as a checked theorem.
#[derive(Debug)]
pub struct LeanSearchCounterexample {
    shared: Arc<Shared>,
}

impl LeanSearchCounterexample {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    from: i64,
    to: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    predicate: String,
    range: Range,
}

/// A witness the service found, and what it is a witness for.
#[derive(Debug, Clone, Copy)]
struct Found<'a> {
    source: &'a ProjectPath,
    predicate: &'a str,
    domain: &'a str,
    witness: i64,
}

/// The module name of the generated witness file for `predicate`: each dotted
/// segment capitalised and joined, ASCII alphanumerics only.
fn witness_module_name(predicate: &str) -> String {
    let joined: String = predicate
        .split('.')
        .map(|segment| {
            let mut chars = segment.chars().filter(char::is_ascii_alphanumeric);
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + &chars.collect::<String>())
                .unwrap_or_default()
        })
        .collect();
    match joined.chars().next() {
        Some(c) if c.is_ascii_uppercase() => joined,
        _ => format!("P{joined}"),
    }
}

/// The frame's `domain`: `nat` or `int` as the service decided it, or
/// `null` when the search stopped before the service determined it.
fn domain_value(domain: &str) -> Value {
    if domain.is_empty() {
        Value::Null
    } else {
        Value::String(domain.to_owned())
    }
}

/// What ran out when a service search stopped short, or `None` for a status
/// that is not a budget hit.
fn limit_hit(status: Status) -> Option<&'static str> {
    match status {
        Status::Timeout => Some("ran out of time"),
        Status::Memory => Some("ran out of memory"),
        Status::OutputLimit => Some("passed the output size cap"),
        _ => None,
    }
}

/// The text of the generated file proving `∃ n, predicate n` by `decide`.
fn witness_source(module: &str, predicate: &str, domain: &str, witness: i64) -> String {
    let (ty, term) = if domain == "int" {
        ("ℤ", format!("({witness} : ℤ)"))
    } else {
        ("ℕ", witness.to_string())
    };
    format!(
        "import {module}\n\ntheorem {predicate}_witness : ∃ n : {ty}, {predicate} n := ⟨{term}, by decide⟩\n"
    )
}

impl LeanSearchCounterexample {
    async fn run(&self, ctx: &CallContext, args: Value) -> Result<Envelope> {
        let args: Args = parse_args(args)?;
        let path = parse_lean_path(&args.path)?;
        let predicate = args.predicate.trim();
        if !valid_name(predicate) {
            return Err(Error::InvalidInput(format!(
                "{predicate:?} is not a declaration name"
            )));
        }
        let (from, to) = (args.range.from, args.range.to);
        let count = i128::from(to) - i128::from(from) + 1;
        if count <= 0 {
            return Err(Error::InvalidInput(
                "the range is empty: from is above to".into(),
            ));
        }
        if count > i128::from(self.shared.config.max_range) {
            return Err(Error::BudgetExceeded(format!(
                "a range of {count} values is over the cap of {}",
                self.shared.config.max_range
            )));
        }
        let slug = ctx.project.slug();
        let file = self.shared.workspace.read(slug, &path)?;
        if !declarations(&file.text).iter().any(|d| d.name == predicate) {
            return Err(Error::NotFound(format!(
                "{predicate} is not declared in {path}"
            )));
        }
        let frame = |covered: Option<i64>, domain: &str| {
            json!({"predicate": predicate, "from": from, "to": to,
                   "covered_to": covered, "domain": domain_value(domain)})
        };
        let reply = match self
            .shared
            .client
            .counterexample(&CounterRequest {
                project: slug.to_owned(),
                root: self.shared.workspace.service_root(),
                path: path.to_string(),
                predicate: predicate.to_owned(),
                from,
                to,
                timeout_ms: self.shared.config.check_timeout_ms,
            })
            .await
        {
            Ok(reply) => reply,
            Err(Error::Timeout(_)) => {
                return Ok(Envelope::unknown(
                    "the search ran out of time before covering the range",
                )
                .with_data(json!({"found": false, "result": "limit", "frame": frame(None, "")})));
            }
            Err(other) => return Err(other),
        };
        if reply.status == Status::BadPredicate {
            return Err(Error::InvalidInput(format!(
                "{predicate} is not a decidable predicate on ℕ or ℤ: {}",
                reply.message
            )));
        }
        if let Some(ran_out) = limit_hit(reply.status) {
            return Ok(Envelope::unknown(match reply.covered_to {
                Some(covered) => {
                    format!("the search {ran_out} after covering {from}..={covered}")
                }
                None => format!("the search {ran_out} before covering any value"),
            })
            .with_data(json!({
                "found": false,
                "result": "limit",
                "frame": frame(reply.covered_to, &reply.domain),
            })));
        }
        expect_ok(reply.status, "the search")?;
        let coverage = coverage(
            format!(
                "decide over {} in {from}..={to}",
                if reply.domain == "int" { "ℤ" } else { "ℕ" }
            ),
            &["values outside the range"],
        );
        let Some(witness) = reply.witness else {
            // The whole range was decided and no value satisfies the
            // predicate: a complete answer about that frame, and only it.
            return Ok(Envelope::ok(json!({
                "found": false,
                "result": "exhausted",
                "frame": frame(Some(to), &reply.domain),
            }))
            .with_coverage(coverage));
        };
        let found = Found {
            source: &path,
            predicate,
            domain: &reply.domain,
            witness,
        };
        self.record_witness(ctx, &found, &frame(Some(witness), &reply.domain), coverage)
            .await
    }

    /// Writes the generated witness file, checks it, and builds the answer.
    async fn record_witness(
        &self,
        ctx: &CallContext,
        found: &Found<'_>,
        frame: &Value,
        coverage: Coverage,
    ) -> Result<Envelope> {
        let Found {
            source,
            predicate,
            domain,
            witness,
        } = *found;
        let name = witness_module_name(predicate);
        let target = ProjectPath::parse(&format!("lean/Supermath/Witnesses/{name}.lean"))?;
        let content = witness_source(&source.module(), predicate, domain, witness);
        let theorem = format!("{predicate}_witness");
        let mut data = json!({
            "found": true,
            "witness": witness,
            "frame": frame,
            "witness_file": target.as_str(),
            "theorem": theorem,
            "evidence_origin": ctx.evidence_origin,
            "verdict": Verdict::Unknown,
            "provenance": null,
            "errors": [],
        });
        if let Err(why) = self.write_witness(ctx, &target, &content) {
            // The witness stands on the service's search; only the theorem
            // recording it is missing.
            data["witness_file"] = Value::Null;
            return Ok(Envelope::ok(data)
                .with_coverage(coverage)
                .with_unknown(format!(
                    "the witness file {target} could not be written ({why}); the theorem is not \
                     evidence yet"
                )));
        }
        let checked = self
            .shared
            .checker
            .check(
                ctx.project.slug(),
                &target,
                Some(std::slice::from_ref(&theorem)),
                self.shared.config.check_timeout_ms,
            )
            .await;
        let report = match checked {
            Ok(outcome) => outcome.report,
            Err(why) => {
                return Ok(Envelope::ok(data)
                    .with_coverage(coverage)
                    .with_unknown(format!(
                        "the witness theorem could not be checked ({why}); it is not evidence yet"
                    )));
            }
        };
        data["verdict"] = json!(report.verdict);
        data["provenance"] = json!(report.provenance);
        data["errors"] = json!(report.errors);
        let mut envelope = Envelope::ok(data).with_coverage(coverage);
        if report.verdict != Verdict::Checked {
            envelope = envelope.with_unknown(format!(
                "the witness theorem did not check (verdict {}); it is not evidence yet",
                report.verdict.as_str()
            ));
        }
        Ok(envelope)
    }

    /// Writes the generated witness file, leaving an identical one alone and
    /// replacing any other: the theorem's statement does not depend on the
    /// witness, so a newer witness proves the same thing.
    fn write_witness(&self, ctx: &CallContext, target: &ProjectPath, content: &str) -> Result<()> {
        let slug = ctx.project.slug();
        let existing = if self.shared.workspace.exists(slug, target)? {
            Some(self.shared.workspace.sha256(slug, target)?)
        } else {
            None
        };
        if existing.as_deref() != Some(sha256_hex(content.as_bytes()).as_str()) {
            let meta = WriteMeta::new("lean_search_counterexample", ctx.call_id.clone())
                .allowing_generated();
            self.shared
                .workspace
                .write(slug, target, content, existing.as_deref(), &meta)?;
        }
        Ok(())
    }
}

#[async_trait]
impl Tool for LeanSearchCounterexample {
    fn name(&self) -> &'static str {
        "lean_search_counterexample"
    }

    fn description(&self) -> &'static str {
        "Given a Lean file that declares a decidable predicate on naturals or integers and an \
         inclusive range, searches the range exhaustively for a value satisfying it. A witness is \
         recorded as a checked theorem in a generated file under lean/Supermath/Witnesses/ and \
         returned with its verdict; a range fully searched with no witness is ok and exhausted, stating the frame and nothing beyond it; a search cut short is unknown, never a negative answer. \
         The range is capped at 1,000,000 values."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "The Lean module declaring the predicate"},
                "predicate": {"type": "string", "description": "The predicate's full name, e.g. Euler.notPrime"},
                "range": {
                    "type": "object",
                    "properties": {"from": {"type": "integer"}, "to": {"type": "integer"}},
                    "required": ["from", "to"],
                    "additionalProperties": false
                }
            },
            "required": ["path", "predicate", "range"],
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
        guard(ctx, self.run(ctx, args)).await
    }
}

#[cfg(test)]
mod test;
