//! Types a host hands a tool: the project, the origin of evidence, and the
//! cancellation signal.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::error::{Error, Result};

/// A project: the slug that names it and the memory scope root that holds its
/// state. Set by the host, never taken from tool arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Project {
    slug: String,
    scope: String,
}

impl Project {
    /// Builds a project. Only a host calls this; tools receive one in their
    /// [`CallContext`](crate::CallContext).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidProjectSlug`] unless `slug` is non-empty and made of
    /// `[a-z0-9-]`; [`Error::EmptyProjectScope`] if `scope` is empty.
    pub fn new(slug: impl Into<String>, scope: impl Into<String>) -> Result<Self> {
        let slug = slug.into();
        let scope = scope.into();
        let valid = !slug.is_empty()
            && slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !valid {
            return Err(Error::InvalidProjectSlug(slug));
        }
        if scope.is_empty() {
            return Err(Error::EmptyProjectScope);
        }
        Ok(Self { slug, scope })
    }

    /// The project's slug, e.g. `twin-primes`.
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The scope root that holds the project's state.
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

/// Where a piece of evidence came from. Only [`EvidenceOrigin::Collected`]
/// evidence may set a claim to `checked` or `formalised`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    /// A tool in this process produced it.
    Collected,
    /// It arrived as text.
    Supplied,
}

/// A cloneable cancellation signal. All clones share one state.
#[derive(Debug, Clone)]
pub struct Cancellation {
    tx: Arc<watch::Sender<bool>>,
}

impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}

impl Cancellation {
    /// A signal that has not fired.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tx: Arc::new(watch::channel(false).0),
        }
    }

    /// Fires the signal for every clone.
    pub fn cancel(&self) {
        self.tx.send_replace(true);
    }

    /// Whether the signal has fired.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        *self.tx.borrow()
    }

    /// Completes once the signal fires; immediately if it already has.
    pub async fn cancelled(&self) {
        let mut rx = self.tx.subscribe();
        // The sender lives in `self`, so the channel cannot close here.
        let _ = rx.wait_for(|fired| *fired).await;
    }
}
