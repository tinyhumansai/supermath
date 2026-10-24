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
//! | `CallLog` | `supermath-tool` | one log every dispatch records into: an [`InMemoryCallLog`], or the project's `FileCallLog` for [`Toolkit::compose_persistent`] |
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
use std::time::{SystemTime, UNIX_EPOCH};

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
    calls: Arc<dyn CallLog>,
    call_prefix: String,
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
        Self::build(config, project, Persistence::Memory)
    }

    /// As [`Toolkit::compose`], with the call log on disk at the project's
    /// `.log/calls.jsonl` instead of in memory, and call ids unique across
    /// processes (`call-<session>-<n>`). Hosts that run each call as its own
    /// process use this, so a `ledger_record` in one process can cite a call
    /// another process made.
    ///
    /// ```
    /// use supermath_mcp::{Config, Toolkit};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> supermath_mcp::Result<()> {
    /// let dir = tempfile::tempdir().unwrap();
    /// let mut config = Config::from_lookup(|_| None)?;
    /// config.files.projects = dir.path().to_path_buf();
    /// let first = Toolkit::compose_persistent(&config, "demo")?;
    /// let done = first.dispatch("calc_factor", serde_json::json!({"n": 12})).await;
    ///
    /// // A toolkit composed later, as by another process, finds the call.
    /// let second = Toolkit::compose_persistent(&config, "demo")?;
    /// assert!(second.call_log().get(&done.call_id).is_some());
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Toolkit::compose`].
    pub fn compose_persistent(config: &Config, project: &str) -> Result<Self> {
        Self::build(config, project, Persistence::Disk)
    }

    fn build(config: &Config, project: &str, persistence: Persistence) -> Result<Self> {
        let slug = slugify(project).ok_or_else(|| Error::InvalidProject(project.to_owned()))?;
        let scope = format!("{}/project:{slug}", config.ledger.scope_root);
        let project = Project::new(slug, scope)?;

        let workspace = Arc::new(config.files.open());
        let (calls, call_prefix): (Arc<dyn CallLog>, String) = match persistence {
            Persistence::Memory => (Arc::new(InMemoryCallLog::new()), "call-".to_owned()),
            Persistence::Disk => (
                Arc::new(workspace.call_log(project.slug())?),
                format!("call-{}-", session()),
            ),
        };
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
        let ledger = Arc::new(Ledger::new(memory.clone(), evidence, calls.clone()));

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
            call_prefix,
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
        let n = self.next_call.fetch_add(1, Ordering::Relaxed);
        format!("{}{n}", self.call_prefix)
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

/// Where a toolkit's call log lives.
#[derive(Debug, Clone, Copy)]
enum Persistence {
    /// In this process only.
    Memory,
    /// In the project's `.log/calls.jsonl`.
    Disk,
}

/// A token telling this process's calls apart from every other process's:
/// the start time in milliseconds and the process id, in hex. Two processes
/// would have to share both to collide.
fn session() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    format!("{millis:x}{:x}", std::process::id())
}

#[cfg(test)]
mod test;
