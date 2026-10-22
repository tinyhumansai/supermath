//! The eight `lean_*` tools and the [`Lean`] bundle that builds them.

mod lean_blueprint;
mod lean_build;
mod lean_check;
mod lean_decl;
mod lean_goals;
mod lean_search;
mod lean_search_counterexample;
mod lean_try_tactics;
mod project;
mod wire;

use std::fmt;
use std::sync::Arc;

use supermath_files::Workspace;
use supermath_tool::Tool;

use crate::checker::{LeanChecker, VerdictCache};
use crate::client::{Health, LeanClient};
use crate::config::LeanConfig;
use crate::error::Result;
use crate::hosted::HostedSearch;
use crate::observer::LeanObserver;

pub use lean_blueprint::{ClaimIndex, LeanBlueprint};
pub use lean_build::LeanBuild;
pub use lean_check::LeanCheck;
pub use lean_decl::LeanDecl;
pub use lean_goals::LeanGoals;
pub use lean_search::LeanSearch;
pub use lean_search_counterexample::LeanSearchCounterexample;
pub use lean_try_tactics::LeanTryTactics;

/// What every tool holds: the workspace, the clients, the checker, and the
/// settings. Built once by [`Lean`] and shared.
pub(crate) struct Shared {
    pub(crate) workspace: Arc<Workspace>,
    pub(crate) client: Arc<LeanClient>,
    pub(crate) checker: LeanChecker,
    pub(crate) hosted: HostedSearch,
    pub(crate) config: LeanConfig,
    pub(crate) claims: Option<Arc<dyn ClaimIndex>>,
}

impl fmt::Debug for Shared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shared")
            .field("client", &self.client)
            .field("config", &self.config)
            .field("claims", &self.claims.is_some())
            .finish_non_exhaustive()
    }
}

/// The Lean toolkit: tools, checker, observer, and client, built once from a
/// [`LeanConfig`] and the project [`Workspace`].
///
/// ```no_run
/// # fn demo() -> supermath_lean::Result<()> {
/// use std::sync::Arc;
/// use supermath_files::{FilesConfig, Workspace};
/// use supermath_lean::{Lean, LeanConfig};
///
/// let workspace = Arc::new(Workspace::new(FilesConfig::from_env().projects));
/// let lean = Lean::new(LeanConfig::from_env(), workspace)?;
/// let names: Vec<_> = lean.tools().iter().map(|t| t.name()).collect();
/// assert_eq!(names.len(), 8);
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct Lean {
    shared: Arc<Shared>,
    cache: Arc<VerdictCache>,
}

impl Lean {
    /// Builds the clients, the checker, and the verdict cache.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Unavailable`] if an HTTP client cannot be built.
    pub fn new(config: LeanConfig, workspace: Arc<Workspace>) -> Result<Self> {
        Self::build(config, workspace, None)
    }

    /// As [`Lean::new`], with the ledger's claim ids for `lean_blueprint`.
    ///
    /// # Errors
    ///
    /// As [`Lean::new`].
    pub fn with_claim_index(
        config: LeanConfig,
        workspace: Arc<Workspace>,
        claims: Arc<dyn ClaimIndex>,
    ) -> Result<Self> {
        Self::build(config, workspace, Some(claims))
    }

    fn build(
        config: LeanConfig,
        workspace: Arc<Workspace>,
        claims: Option<Arc<dyn ClaimIndex>>,
    ) -> Result<Self> {
        let client = Arc::new(LeanClient::new(&config)?);
        let hosted = HostedSearch::new(&config)?;
        let cache = Arc::new(VerdictCache::default());
        let checker = LeanChecker::new(workspace.clone(), client.clone(), cache.clone());
        Ok(Self {
            shared: Arc::new(Shared {
                workspace,
                client,
                checker,
                hosted,
                config,
                claims,
            }),
            cache,
        })
    }

    /// The eight tools, in catalogue order.
    #[must_use]
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        let s = &self.shared;
        vec![
            Arc::new(LeanCheck::new(s.clone())),
            Arc::new(LeanSearch::new(s.clone())),
            Arc::new(LeanDecl::new(s.clone())),
            Arc::new(LeanGoals::new(s.clone())),
            Arc::new(LeanTryTactics::new(s.clone())),
            Arc::new(LeanSearchCounterexample::new(s.clone())),
            Arc::new(LeanBuild::new(s.clone())),
            Arc::new(LeanBlueprint::new(s.clone())),
        ]
    }

    /// The checker the tools share, for the ledger's evidence adapter.
    #[must_use]
    pub fn checker(&self) -> LeanChecker {
        self.shared.checker.clone()
    }

    /// The observer to register with `file_list`.
    #[must_use]
    pub fn observer(&self) -> Arc<LeanObserver> {
        Arc::new(LeanObserver::new(
            self.shared.workspace.clone(),
            self.cache.clone(),
        ))
    }

    /// The service client.
    #[must_use]
    pub fn client(&self) -> Arc<LeanClient> {
        self.shared.client.clone()
    }

    /// The toolchain and Mathlib revision the service runs: what both `check`
    /// commands report.
    ///
    /// # Errors
    ///
    /// As [`LeanClient::health`].
    pub async fn health(&self) -> Result<Health> {
        self.shared.client.health().await
    }
}

#[cfg(test)]
mod test;
