//! `py_search_counterexample`: search a stated frame for a failing input.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_files::WriteMeta;
use supermath_tool::{
    CallContext, Coverage, Envelope, ErrorCode, Exposure, Kind, Permission, Tool,
};

use super::{Shared, parse_args};
use crate::client::{Execution, Limit, SearchOutcome, SearchRequest};
use crate::error::{Error, Result};
use crate::frame::{Frame, is_identifier};
use crate::provenance::args_sha256;
use crate::{Provenance, witness};

/// Searches a frame for an input where a predicate returns `False`.
#[derive(Debug, Clone)]
pub struct PySearchCounterexample {
    shared: Arc<Shared>,
}

impl PySearchCounterexample {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    predicate: String,
    frame: Value,
    seed: Option<u64>,
    timeout_s: Option<u64>,
}

/// A witness the service found.
struct Hit {
    value: Value,
    witness_json: String,
    checked: u64,
}

/// The facts every branch of the answer reports.
struct Searched<'a> {
    module: &'a str,
    predicate: &'a str,
    frame: &'a Frame,
    execution: &'a Execution,
    provenance: &'a Provenance,
}

impl Searched<'_> {
    fn data(&self, result: &str, extra: &Value) -> Value {
        let mut data = json!({
            "result": result,
            "frame": self.frame.to_wire(),
            "frame_description": self.frame.describe(),
            "stdout": self.execution.output.stdout,
            "stderr": self.execution.output.stderr,
            "provenance": self.provenance,
        });
        if let (Some(target), Some(extra)) = (data.as_object_mut(), extra.as_object()) {
            target.extend(extra.clone());
        }
        data
    }

    fn partial(&self, checked: Option<u64>) -> Value {
        json!({
            "checked": checked,
            "covered": checked.and_then(|n| self.frame.covered(n)),
        })
    }
}

impl PySearchCounterexample {
    async fn run(&self, ctx: &CallContext, args: Args) -> Result<Envelope> {
        let shared = &self.shared;
        if !is_identifier(&args.predicate) {
            return Err(Error::InvalidInput(
                "predicate must name a top-level function".into(),
            ));
        }
        let frame = Frame::parse(&args.frame)?;
        if let Some(size) = frame.size()
            && size > u128::from(shared.config.max_frame)
        {
            return Err(Error::BudgetExceeded(format!(
                "the frame holds {size} values, over the admission cap of {}",
                shared.config.max_frame
            )));
        }
        let timeout_s = shared.timeout(&ctx.budget, args.timeout_s)?;
        let seed = args.seed.unwrap_or(0);
        let slug = ctx.project.slug();
        let prepared = shared.prepare(slug, &args.path)?;
        let request = SearchRequest {
            project: slug.to_owned(),
            path: prepared.path.to_string(),
            predicate: args.predicate.clone(),
            frame: frame.to_wire(),
            seed,
            timeout_s,
            max_frame: shared.config.max_frame,
        };
        let report = tokio::select! {
            report = shared.client.search(&request) => report?,
            () = ctx.cancel.cancelled() => {
                return Ok(Envelope::unknown("the call was cancelled before the search finished"));
            }
        };
        let frame_hash = args_sha256(&json!({
            "frame": frame.to_wire(),
            "max_frame": shared.config.max_frame,
        }));
        let provenance = Shared::provenance(
            &prepared,
            &args.predicate,
            frame_hash,
            seed,
            &report.execution,
        )?;
        let label = match &report.outcome {
            SearchOutcome::Found { .. } => "found",
            SearchOutcome::Exhausted { .. } => "exhausted",
            SearchOutcome::Capped { .. } | SearchOutcome::Limit { .. } => "unknown",
            SearchOutcome::PredicateRaised { .. } | SearchOutcome::Raised { .. } => "raised",
        };
        shared
            .observer
            .record(slug, &prepared.path, label, &prepared.file_sha256);
        let module = prepared.path.module();
        let searched = Searched {
            module: &module,
            predicate: &args.predicate,
            frame: &frame,
            execution: &report.execution,
            provenance: &provenance,
        };
        Ok(self.envelope(ctx, &searched, report.outcome))
    }

    fn envelope(
        &self,
        ctx: &CallContext,
        searched: &Searched<'_>,
        outcome: SearchOutcome,
    ) -> Envelope {
        let shared = &self.shared;
        match outcome {
            SearchOutcome::Found {
                witness,
                witness_json,
                checked,
            } => self.found(
                ctx,
                searched,
                &Hit {
                    value: witness,
                    witness_json,
                    checked,
                },
            ),
            SearchOutcome::Exhausted { checked } => exhausted(ctx, searched, checked),
            SearchOutcome::Capped { checked } => Envelope::unknown(format!(
                "the generator was not exhausted: the search stopped at the admission cap of {} values",
                shared.config.max_frame
            ))
            .with_data(searched.data("capped", &searched.partial(Some(checked)))),
            SearchOutcome::PredicateRaised {
                input,
                exception,
                checked,
            } => Envelope::unknown(format!(
                "the predicate raised on input {input}; a crash is not a counterexample, \
                 and the search stopped there"
            ))
            .with_data(searched.data(
                "predicate_raised",
                &json!({"input": input, "exception": exception, "checked": checked}),
            )),
            SearchOutcome::Raised { exception } => Envelope::unknown(
                "the file raised before the search could run, so nothing was searched",
            )
            .with_data(searched.data("raised", &json!({"exception": exception}))),
            SearchOutcome::Limit { limit, checked } => limited(searched, limit, checked),
        }
    }

    fn found(&self, ctx: &CallContext, searched: &Searched<'_>, hit: &Hit) -> Envelope {
        let source = witness::render(
            searched.module,
            searched.predicate,
            &hit.witness_json,
            searched.frame,
        );
        let relative = witness::path_for(searched.predicate, &source);
        let written = self.write_witness(ctx, &relative, &source);
        let mut extra = json!({
            "witness": hit.value,
            "checked_before": hit.checked,
        });
        let mut next = Vec::new();
        let mut unknowns = Vec::new();
        match written {
            Ok(sha256) => {
                extra["witness_file"] = json!({"path": relative, "sha256": sha256});
                next.push(format!(
                    "py_run {{\"path\": \"{relative}\", \"entry\": \"check\"}} re-runs the predicate on the witness"
                ));
            }
            Err(error) => {
                extra["witness_file"] = Value::Null;
                unknowns.push(format!("the witness file could not be written: {error}"));
            }
        }
        next.push(
            "confirm the witness in Lean (lean_search_counterexample, or decide on it) \
             before it refutes anything; Python alone never makes a claim formalised"
                .to_owned(),
        );
        let mut envelope = Envelope::ok(searched.data("found", &extra)).with_next(next);
        for unknown in unknowns {
            envelope = envelope.with_unknown(unknown);
        }
        envelope
    }

    /// Writes the generated witness file; an identical file already there is
    /// fine (the name is content-addressed), anything else is reported.
    fn write_witness(&self, ctx: &CallContext, relative: &str, source: &str) -> Result<String> {
        let workspace = &self.shared.workspace;
        let slug = ctx.project.slug();
        let path = supermath_files::ProjectPath::parse(relative)?;
        if workspace.exists(slug, &path)? {
            return Ok(workspace.sha256(slug, &path)?);
        }
        let meta = WriteMeta::new("py_search_counterexample", &ctx.call_id).allowing_generated();
        Ok(workspace.write(slug, &path, source, None, &meta)?.sha256)
    }
}

fn exhausted(ctx: &CallContext, searched: &Searched<'_>, checked: u64) -> Envelope {
    let description = searched.frame.describe();
    // The whole frame was searched and held no witness: a complete answer
    // about that frame, and only it. `unknown` is reserved for a search cut
    // short.
    Envelope::ok(searched.data("exhausted", &json!({"checked": checked})))
        .with_coverage(Coverage::new(
            [description.clone()],
            ["values outside the frame".to_owned()],
        ))
        .with_next([format!(
            "ledger_record {{\"kind\": \"bound\", \"statement\": \"no counterexample in {description}\", \
             \"frame\": {}, \"source\": {{\"tool\": {{\"tool\": \"py_search_counterexample\", \
             \"call_id\": \"{}\"}}}}}} records it as checked on that frame only",
            Value::String(description.clone()),
            ctx.call_id
        )])
}

fn limited(searched: &Searched<'_>, limit: Limit, checked: Option<u64>) -> Envelope {
    let covered = checked.map_or_else(
        || "how far it got is not known".to_owned(),
        |n| format!("{n} values were checked before it stopped"),
    );
    Envelope::unknown(format!(
        "{} ({}); {covered}, and a limit says nothing about the rest of the frame",
        limit.explanation(),
        limit.as_str()
    ))
    .with_data(searched.data(
        "limit",
        &json!({"limit": limit.as_str(), "checked": checked, "covered": checked.and_then(|n| searched.frame.covered(n))}),
    ))
}

#[async_trait]
impl Tool for PySearchCounterexample {
    fn name(&self) -> &'static str {
        "py_search_counterexample"
    }

    fn description(&self) -> &'static str {
        "Given a predicate function (one argument, returns bool) in a Python \
         file under python/work/ and a finite frame (a range, a product of \
         ranges, or a generator), searches the frame in the sandbox for an \
         input where the predicate is false. A witness is returned and \
         written to python/work/witnesses/; covering the whole frame without \
         a witness is ok and exhausted, stating the exact frame; a search cut \
         short is unknown. A witness found here is confirmed in \
         Lean before it refutes anything."
    }

    fn schema(&self) -> Value {
        let bound = json!({
            "type": "object",
            "properties": {
                "from": { "type": "integer" },
                "to": { "type": "integer" }
            },
            "required": ["from", "to"],
            "additionalProperties": false
        });
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "predicate": { "type": "string" },
                "frame": {
                    "type": "object",
                    "properties": {
                        "kind": { "enum": ["range", "product", "generator"] },
                        "from": { "type": "integer" },
                        "to": { "type": "integer" },
                        "ranges": { "type": "array", "items": bound, "minItems": 1, "maxItems": 8 },
                        "entry": { "type": "string" }
                    },
                    "required": ["kind"],
                    "additionalProperties": false
                },
                "seed": { "type": "integer", "minimum": 0 },
                "timeout_s": { "type": "integer", "minimum": 1 }
            },
            "required": ["path", "predicate", "frame"],
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
        let result = match parse_args::<Args>(args) {
            Ok(args) => self.run(ctx, args).await,
            Err(error) => Err(error),
        };
        result.unwrap_or_else(Envelope::from)
    }
}
