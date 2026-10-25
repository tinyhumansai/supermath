//! `py_run`: call one function of a project file in the sandboxed service.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

use super::{MAX_ARGS_BYTES, Shared, contains_float, parse_args};
use crate::client::{Execution, RunOutcome, RunRequest};
use crate::error::{Error, Result};
use crate::frame::is_identifier;
use crate::provenance::{args_sha256, canonical_json};

/// Runs a function from a project file.
#[derive(Debug, Clone)]
pub struct PyRun {
    shared: Arc<Shared>,
}

impl PyRun {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
    entry: String,
    #[serde(rename = "args")]
    kwargs: Option<Value>,
    seed: Option<u64>,
    timeout_s: Option<u64>,
}

impl PyRun {
    async fn run(&self, ctx: &CallContext, args: Args) -> Result<Envelope> {
        let shared = &self.shared;
        if !is_identifier(&args.entry) {
            return Err(Error::InvalidInput(
                "entry must name a top-level function".into(),
            ));
        }
        let kwargs = match args.kwargs {
            None | Some(Value::Null) => json!({}),
            Some(value @ Value::Object(_)) => value,
            Some(_) => return Err(Error::InvalidInput("args must be a JSON object".into())),
        };
        let canonical = canonical_json(&kwargs);
        if canonical.len() > MAX_ARGS_BYTES {
            return Err(Error::InvalidInput("args are over 64 KB".into()));
        }
        let timeout_s = shared.timeout(&ctx.budget, args.timeout_s)?;
        let seed = args.seed.unwrap_or(0);
        let slug = ctx.project.slug();
        let prepared = shared.prepare(slug, &args.path)?;
        let request = RunRequest {
            project: slug.to_owned(),
            root: shared.workspace.service_root(),
            path: prepared.path.to_string(),
            entry: args.entry.clone(),
            args: kwargs.clone(),
            seed,
            timeout_s,
        };
        let report = tokio::select! {
            report = shared.client.run(&request) => report?,
            () = ctx.cancel.cancelled() => {
                return Ok(Envelope::unknown("the call was cancelled before the run finished"));
            }
        };
        let provenance = Shared::provenance(
            &prepared,
            &args.entry,
            args_sha256(&kwargs),
            seed,
            &report.execution,
        )?;
        let label = match &report.outcome {
            RunOutcome::Returned { .. } => "returned",
            RunOutcome::Raised { .. } => "raised",
            RunOutcome::Limit { .. } => "unknown",
        };
        shared
            .observer
            .record(slug, &prepared.path, label, &prepared.file_sha256);
        Ok(envelope(
            ctx,
            &report.outcome,
            &report.execution,
            &provenance,
        ))
    }
}

fn envelope(
    ctx: &CallContext,
    outcome: &RunOutcome,
    execution: &Execution,
    provenance: &crate::Provenance,
) -> Envelope {
    let output = json!({
        "stdout": execution.output.stdout,
        "stdout_truncated": execution.output.stdout_truncated,
        "stderr": execution.output.stderr,
        "stderr_truncated": execution.output.stderr_truncated,
        "provenance": provenance,
    });
    let with_output = |mut data: Value| {
        if let (Some(target), Some(extra)) = (data.as_object_mut(), output.as_object()) {
            target.extend(extra.clone());
        }
        data
    };
    match outcome {
        RunOutcome::Returned { value } => {
            let exact = value.get("exact") == Some(&Value::Bool(true));
            let mut envelope = Envelope::ok(with_output(json!({
                "result": "returned",
                "value": value,
                "exact": exact,
            })))
            .with_next([format!(
                "ledger_record {{\"kind\": \"bound\", \"frame\": \"<what this run covered>\", \
                 \"source\": {{\"tool\": {{\"tool\": \"py_run\", \"call_id\": \"{}\"}}}}}}",
                ctx.call_id
            )]);
            if !exact && contains_float(value) {
                envelope = envelope.with_unknown(
                    "the value holds floating-point numbers and is not declared exact; \
                     it cannot back an exact claim as checked",
                );
            }
            envelope
        }
        RunOutcome::Raised { exception } => Envelope::ok(with_output(json!({
            "result": "raised",
            "exception": exception,
        }))),
        RunOutcome::Limit { limit } => Envelope::unknown(format!(
            "{} ({}); a limit says nothing about the mathematics",
            limit.explanation(),
            limit.as_str()
        ))
        .with_data(with_output(json!({
            "result": "limit",
            "limit": limit.as_str(),
        }))),
    }
}

#[async_trait]
impl Tool for PyRun {
    fn name(&self) -> &'static str {
        "py_run"
    }

    fn description(&self) -> &'static str {
        "Given a Python file under python/work/, an entry function, and JSON \
         keyword arguments, runs it in the pinned, sandboxed solver \
         environment and returns its JSON value, printed output, and \
         provenance (file, imports, arguments, seed, Python and lock \
         digests). A raised exception is a result; a timeout or memory limit \
         is unknown, never a negative answer. A result is computation, not \
         proof: it can be recorded as checked on a stated frame, never \
         formalised."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "entry": { "type": "string" },
                "args": { "type": "object" },
                "seed": { "type": "integer", "minimum": 0 },
                "timeout_s": { "type": "integer", "minimum": 1 }
            },
            "required": ["path", "entry"],
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
