//! [`Toolkit`]: every primitive domain, composed for one project.
//!
//! [`Toolkit::compose`] builds the domains from a [`Config`], wires their
//! ports to each other, and registers all of their tools in one
//! [`ToolRegistry`]:
//!
//! | Port | Defined in | Wired to |
//! | --- | --- | --- |
//! | `FileObserver` (Lean) | `supermath-files` | the Lean verdict cache |
//! | `FileObserver` (Python) | `supermath-files` | the Python run log |
//! | `LeanEvidence` | `supermath-ledger` | [`LeanBridge`](crate::LeanBridge), over the Lean checker |
//! | `ClaimIndex` | `supermath-lean` | [`RecordedClaims`](crate::RecordedClaims), fed by `ledger_record` |
//! | `CallLog` | `supermath-tool` | one [`InMemoryCallLog`] every dispatch records into |
//!
//! The registry holds the domain crates' tools, `web_search` and
//! `web_research` (which call Gemini) among them; it holds no `agent_*` tool,
//! because those need `OpenHuman` and live in `supermath-subagents`.
//!
//! A domain whose service is not configured still registers its tools, so
//! the catalogue never changes shape with the environment; they answer
//! `BACKEND_UNAVAILABLE`.
//!
//! Calls go through [`Toolkit::dispatch`], which builds the
//! [`CallContext`](supermath_tool::CallContext) from the project the toolkit
//! was composed for (never from arguments) and records the call, so the
//! ledger can later check a `tool` source against the log.

mod types;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;
use supermath_calc::{CalcFactor, CalcGcdLcm};
use supermath_files::{FileObserver, Workspace, file_tools};
use supermath_lean::Lean;
use supermath_ledger::{Ledger, Memory, slugify};
use supermath_literature::Literature;
use supermath_python::Python;
use supermath_tool::{
    CallContext, CallLog, Envelope, InMemoryCallLog, Project, Tool, ToolRegistry,
};
use supermath_web::WebToolkit;

use crate::claims::{RecordedClaims, RecordsClaims};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::evidence::LeanBridge;

pub use types::Dispatched;

/// The name of the ledger tool whose Lean claims feed the claim index.
const LEDGER_RECORD: &str = "ledger_record";

/// Every primitive tool, composed for one project.
#[derive(Debug)]
pub struct Toolkit {
    project: Project,
    registry: ToolRegistry,
    calls: Arc<InMemoryCallLog>,
    next_call: AtomicU64,
    lean: Lean,
    python: Python,
    literature: Literature,
    memory: Memory,
    web_has_key: bool,
}

impl Toolkit {
    /// Builds every domain from `config` and wires their ports, for the
    /// project named `project`. The name is reduced to a slug (lowercase
    /// letters, digits, dashes) the way the ledger does, and the slug is the
    /// project every call acts on.
    ///
    /// ```
    /// use supermath_mcp::{Config, Toolkit};
    ///
    /// # fn main() -> supermath_mcp::Result<()> {
    /// let config = Config::from_lookup(|_| None)?;
    /// let toolkit = Toolkit::compose(&config, "Twin Primes")?;
    /// assert_eq!(toolkit.project().slug(), "twin-primes");
    /// assert!(toolkit.registry().get("calc_factor").is_some());
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// [`Error::InvalidProject`] if `project` has no usable characters;
    /// the domain errors if a domain cannot build its clients;
    /// [`Error::Tool`] if two domains register one tool name.
    pub fn compose(config: &Config, project: &str) -> Result<Self> {
        let slug = slugify(project).ok_or_else(|| Error::InvalidProject(project.to_owned()))?;
        let scope = format!("{}/project:{slug}", config.ledger.scope_root);
        let project = Project::new(slug, scope)?;

        let workspace = Arc::new(Workspace::new(config.files.projects.clone()));
        let calls = Arc::new(InMemoryCallLog::new());
        let claims = Arc::new(RecordedClaims::new());

        let lean = Lean::with_claim_index(config.lean.clone(), workspace.clone(), claims.clone())?;
        let python = Python::new(&config.python, workspace.clone())?;
        let observers: Vec<Arc<dyn FileObserver>> = vec![lean.observer(), python.observer()];

        let memory = config.ledger.memory();
        let evidence = Arc::new(LeanBridge::new(
            lean.checker(),
            project.slug(),
            config.lean.check_timeout_ms,
        ));
        let ledger = Arc::new(Ledger::new(
            memory.clone(),
            evidence,
            calls.clone() as Arc<dyn CallLog>,
        ));

        let literature = Literature::new(&config.literature)?;
        let web = WebToolkit::new(config.web.clone())?;

        let mut tools: Vec<Arc<dyn Tool>> = file_tools(&workspace, observers);
        tools.extend(lean.tools());
        tools.extend(python.tools());
        tools.extend(literature.tools());
        tools.extend(web.tools());
        tools.extend(
            supermath_ledger::tools(&memory, &ledger)
                .into_iter()
                .map(|tool| {
                    if tool.name() == LEDGER_RECORD {
                        Arc::new(RecordsClaims::new(tool, claims.clone())) as Arc<dyn Tool>
                    } else {
                        tool
                    }
                }),
        );
        tools.push(Arc::new(CalcFactor));
        tools.push(Arc::new(CalcGcdLcm));

        let mut registry = ToolRegistry::new();
        for tool in tools {
            registry.register(tool)?;
        }
        Ok(Self {
            project,
            registry,
            calls,
            next_call: AtomicU64::new(1),
            lean,
            python,
            literature,
            memory,
            web_has_key: config.web.has_api_key(),
        })
    }

    /// The project every call acts on.
    #[must_use]
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The tools, for `supermath-subagents` and for the MCP server. Cheap to
    /// clone: it shares the tools.
    #[must_use]
    pub fn registry(&self) -> ToolRegistry {
        self.registry.clone()
    }

    /// The call log every [`Toolkit::dispatch`] writes to and the ledger
    /// reads.
    #[must_use]
    pub fn call_log(&self) -> Arc<dyn CallLog> {
        self.calls.clone()
    }

    /// The id the next call will have.
    fn next_call_id(&self) -> String {
        format!("call-{}", self.next_call.fetch_add(1, Ordering::Relaxed))
    }

    /// A context for a call to this project with the given id: collected
    /// evidence, the default budget.
    #[must_use]
    pub fn context(&self, call_id: impl Into<String>) -> CallContext {
        CallContext::new(self.project.clone(), call_id)
    }

    /// Calls `name` with `args` in a fresh context, records the call in the
    /// call log, and returns the envelope with the call's id. An unknown
    /// name answers `NOT_FOUND`.
    pub async fn dispatch(&self, name: &str, args: Value) -> Dispatched {
        let call_id = self.next_call_id();
        let envelope = self
            .dispatch_with(&self.context(call_id.clone()), name, args)
            .await;
        Dispatched { call_id, envelope }
    }

    /// As [`Toolkit::dispatch`], with a context the caller built (a runner
    /// passes its own call ids, budgets, and cancellation).
    pub async fn dispatch_with(&self, ctx: &CallContext, name: &str, args: Value) -> Envelope {
        self.registry
            .dispatch(self.calls.as_ref(), name, ctx, args)
            .await
    }

    /// The Lean toolkit, for `check` and for hosts that read its client.
    #[must_use]
    pub fn lean(&self) -> &Lean {
        &self.lean
    }

    /// The Python toolkit.
    #[must_use]
    pub fn python(&self) -> &Python {
        &self.python
    }

    /// The literature toolkit.
    #[must_use]
    pub fn literature(&self) -> &Literature {
        &self.literature
    }

    /// The `CortexDB` client the ledger writes through.
    #[must_use]
    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// Whether a Gemini key is configured (never the key itself).
    #[must_use]
    pub fn has_gemini_key(&self) -> bool {
        self.web_has_key
    }
}

#[cfg(test)]
mod test;
