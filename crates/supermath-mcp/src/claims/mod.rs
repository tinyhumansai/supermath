//! [`RecordedClaims`]: the ledger's claim ids for `lean_blueprint`, learned
//! from `ledger_record`.
//!
//! `lean_blueprint` marks each declaration with the ledger claim that rests
//! on it. Its port, [`ClaimIndex`], is synchronous, while the ledger lives
//! behind an async HTTP memory server, so the port cannot query the ledger
//! directly. Instead [`RecordsClaims`] wraps the `ledger_record` tool and
//! remembers, for every successful call with a `lean` source, the claim id the
//! ledger returned.
//!
//! The index therefore covers the claims recorded through this toolkit since
//! the process started. A claim recorded earlier, or by another process, is
//! absent until it is recorded again; recording is idempotent, so doing so
//! changes nothing in the ledger. The index only names claims; it never
//! decides a status.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use serde_json::Value;
use supermath_lean::ClaimIndex;
use supermath_tool::{CallContext, Envelope, ErrorCode, Exposure, Kind, Permission, Tool};

/// `(project, path, declaration)`.
type Key = (String, String, String);

/// The claim ids recorded through this toolkit, by Lean declaration.
#[derive(Debug, Default)]
pub struct RecordedClaims {
    ids: Mutex<BTreeMap<Key, String>>,
}

impl RecordedClaims {
    /// An empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Notes that claim `id` rests on `declaration` in `path` of `project`.
    /// A later claim on the same declaration replaces the earlier one.
    pub fn insert(&self, project: &str, path: &str, declaration: &str, id: &str) {
        self.ids
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(
                (project.to_owned(), path.to_owned(), declaration.to_owned()),
                id.to_owned(),
            );
    }
}

impl ClaimIndex for RecordedClaims {
    fn claim_id(&self, project: &str, path: &str, declaration: &str) -> Option<String> {
        self.ids
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&(project.to_owned(), path.to_owned(), declaration.to_owned()))
            .cloned()
    }
}

/// `ledger_record`, unchanged, that also feeds a [`RecordedClaims`].
pub struct RecordsClaims {
    inner: Arc<dyn Tool>,
    index: Arc<RecordedClaims>,
}

impl std::fmt::Debug for RecordsClaims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordsClaims")
            .field("inner", &self.inner.name())
            .finish_non_exhaustive()
    }
}

impl RecordsClaims {
    /// Wraps `inner` so each recorded Lean claim lands in `index`.
    #[must_use]
    pub fn new(inner: Arc<dyn Tool>, index: Arc<RecordedClaims>) -> Self {
        Self { inner, index }
    }
}

/// The `(path, declaration)` of a `lean` source in `ledger_record` arguments.
fn lean_source(args: &Value) -> Option<(String, String)> {
    let lean = args.get("source")?.get("lean")?;
    Some((
        lean.get("path")?.as_str()?.to_owned(),
        lean.get("declaration")?.as_str()?.to_owned(),
    ))
}

#[async_trait]
impl Tool for RecordsClaims {
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

    fn errors(&self) -> &'static [ErrorCode] {
        self.inner.errors()
    }

    async fn call(&self, ctx: &CallContext, args: Value) -> Envelope {
        let source = lean_source(&args);
        let envelope = self.inner.call(ctx, args).await;
        if let (Some((path, declaration)), Some(id)) =
            (source, envelope.data.get("id").and_then(Value::as_str))
        {
            self.index
                .insert(ctx.project.slug(), &path, &declaration, id);
        }
        envelope
    }
}

#[cfg(test)]
mod test;
