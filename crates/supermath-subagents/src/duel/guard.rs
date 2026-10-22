//! [`WriteGuard`]: `file_write` and `file_edit` confined to one side's area.
//!
//! The two sides share one project workspace. Each may read anything there,
//! but writes only under its own prefixes, so neither can touch the pinned
//! statement, the host's verdict files, or the other side's work, and the two
//! never race for one file's `if_sha256`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

/// Whether `path` lies under one of `areas` without escaping it.
pub(crate) fn within(path: &str, areas: &[String]) -> bool {
    let clean = !path.contains('\\')
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."));
    clean && areas.iter().any(|area| path.starts_with(area.as_str()))
}

/// A write tool that refuses any path outside its areas.
pub(crate) struct WriteGuard {
    inner: Arc<dyn Tool>,
    areas: Vec<String>,
}

impl WriteGuard {
    /// Wraps `inner`, a tool whose `path` argument names what it writes.
    pub(crate) fn new(inner: Arc<dyn Tool>, areas: Vec<String>) -> Self {
        Self { inner, areas }
    }
}

#[async_trait]
impl Tool for WriteGuard {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn description(&self) -> &'static str {
        self.inner.description()
    }

    fn schema(&self) -> Value {
        self.inner.schema()
    }

    fn exposure(&self) -> Exposure {
        self.inner.exposure()
    }

    fn permission(&self) -> Permission {
        self.inner.permission()
    }

    fn kind(&self) -> Kind {
        self.inner.kind()
    }

    /// What `file_write` and `file_edit` can answer, and the guard's own
    /// `FORBIDDEN`.
    fn errors(&self) -> &'static [ErrorCode] {
        &[
            ErrorCode::InvalidInput,
            ErrorCode::NotFound,
            ErrorCode::Conflict,
            ErrorCode::BudgetExceeded,
            ErrorCode::BackendUnavailable,
            ErrorCode::Forbidden,
        ]
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let path = args.get("path").and_then(Value::as_str).unwrap_or_default();
        if within(path, &self.areas) {
            self.inner.call(ctx, args).await
        } else {
            Envelope::error(
                ErrorCode::Forbidden,
                format!(
                    "in the duel you may write only under {}; `{path}` is outside your area",
                    self.areas.join(" or ")
                ),
            )
        }
    }
}
