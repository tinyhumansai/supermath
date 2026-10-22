//! A scripted [`LeanEvidence`] for tests of the ledger and of its callers.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;

use super::{LeanEvidence, LeanVerdict, LeanVerdictKind};
use crate::error::{Error, Result};

/// Answers `verify` from a table keyed by file path and declaration, and
/// counts the calls it gets.
///
/// A path not in the table is [`Error::EvidenceNotFound`]. The table can be
/// changed between calls, which is how a test edits a file.
#[derive(Debug, Default)]
pub struct FakeLeanEvidence {
    answers: Mutex<BTreeMap<(String, String), Result<LeanVerdict>>>,
    calls: Mutex<Vec<(String, String)>>,
}

impl FakeLeanEvidence {
    /// A fake that knows nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A clean verdict: `Checked`, no `sorry`, no debt, `axioms` for the
    /// declaration, at `file_sha256`.
    #[must_use]
    pub fn clean(file_sha256: &str, declaration: &str, axioms: &[&str]) -> LeanVerdict {
        LeanVerdict {
            verdict: LeanVerdictKind::Checked,
            file_sha256: file_sha256.to_string(),
            sorries: 0,
            debt: Vec::new(),
            axioms: BTreeMap::from([(
                declaration.to_string(),
                axioms.iter().map(|a| (*a).to_string()).collect(),
            )]),
            provenance: serde_json::json!({"toolchain": "fake"}),
        }
    }

    /// Sets the answer for `path` and `declaration`.
    pub fn set(&self, path: &str, declaration: &str, answer: Result<LeanVerdict>) {
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert((path.to_string(), declaration.to_string()), answer);
    }

    /// Every `(path, declaration)` asked for so far, oldest first.
    #[must_use]
    pub fn calls(&self) -> Vec<(String, String)> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl LeanEvidence for FakeLeanEvidence {
    async fn verify(&self, path: &str, declaration: &str) -> Result<LeanVerdict> {
        let key = (path.to_string(), declaration.to_string());
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(key.clone());
        let answers = self.answers.lock().unwrap_or_else(PoisonError::into_inner);
        match answers.get(&key) {
            Some(Ok(verdict)) => Ok(verdict.clone()),
            Some(Err(error)) => Err(clone_error(error)),
            None => Err(Error::EvidenceNotFound {
                path: key.0,
                declaration: key.1,
            }),
        }
    }
}

/// The evidence errors are all the fake ever stores.
fn clone_error(error: &Error) -> Error {
    match error {
        Error::EvidenceNotFound { path, declaration } => Error::EvidenceNotFound {
            path: path.clone(),
            declaration: declaration.clone(),
        },
        Error::EvidenceTimeout => Error::EvidenceTimeout,
        other => Error::EvidenceUnavailable(other.to_string()),
    }
}
