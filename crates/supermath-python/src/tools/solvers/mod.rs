//! `py_solvers`: what the pinned environment can import.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use supermath_tool::{
    CallContext, Coverage, Envelope, ErrorCode, Exposure, Kind, Permission, Tool,
};

use super::{Shared, parse_args};
use crate::error::{Error, Result};

/// The areas a solver is filed under.
pub(crate) const AREAS: [&str; 6] = [
    "symbolic",
    "numeric",
    "sat",
    "smt",
    "optimization",
    "graphs",
];

/// Lists the solvers in the lock.
#[derive(Debug, Clone)]
pub struct PySolvers {
    shared: Arc<Shared>,
}

impl PySolvers {
    pub(super) fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    area: Option<String>,
}

impl PySolvers {
    async fn run(&self, args: Args) -> Result<Envelope> {
        if let Some(area) = &args.area
            && !AREAS.contains(&area.as_str())
        {
            return Err(Error::InvalidInput(format!(
                "unknown area {area:?}; use one of {}",
                AREAS.join(", ")
            )));
        }
        let list = self.shared.client.solvers().await?;
        let rows: Vec<_> = list
            .solvers
            .iter()
            .filter(|s| args.area.as_deref().is_none_or(|area| s.area == area))
            .collect();
        let lock = list.lock_sha256.as_deref().map_or_else(
            || "uv.lock (digest unavailable)".to_owned(),
            |d| format!("uv.lock {d}"),
        );
        let mut envelope = Envelope::ok(json!({
            "python": list.python,
            "lock_sha256": list.lock_sha256,
            "solvers": rows,
        }))
        .with_coverage(Coverage::new(
            [lock],
            [
                "packages outside the lock cannot be imported".to_owned(),
                "SageMath is not in the environment".to_owned(),
            ],
        ));
        if rows.is_empty() {
            envelope = envelope.with_unknown("no solver in the lock is filed under that area");
        }
        Ok(envelope)
    }
}

#[async_trait]
impl Tool for PySolvers {
    fn name(&self) -> &'static str {
        "py_solvers"
    }

    fn description(&self) -> &'static str {
        "Given nothing, or an area (symbolic, numeric, sat, smt, optimization, \
         graphs), returns the solvers in the pinned Python environment with \
         package, version, import name, area, and a one-line summary, read \
         from the service's lock. Use it before writing a file that imports a \
         solver."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "area": { "type": "string", "enum": AREAS }
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

    async fn call(&self, _ctx: &CallContext, args: Value) -> Envelope {
        let result = match parse_args::<Args>(args) {
            Ok(args) => self.run(args).await,
            Err(error) => Err(error),
        };
        result.unwrap_or_else(Envelope::from)
    }
}
