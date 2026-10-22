//! The claim ledger: what the project knows and how well it knows it.
//!
//! The ledger keeps facts, claims, bounds, leads, and dead ends in
//! `CortexDB`. Every entry has a kind, a status, a source, and edges to the
//! claims it uses. **A status is earned, never given**: no caller can send
//! `formalised` or `collected`, `formalised` needs a clean Lean check this
//! process ran, `checked` needs a successful call this process logged, and a
//! changed file drops its claim back to `asserted`. The rules are in
//! `docs/specs/toolkit/ledger/README.md`.
//!
//! # Entry points
//!
//! - [`Config`] reads `CORTEX_API_URL`, `CORTEX_API_KEY`, and
//!   `CORTEX_SCOPE_ROOT`; [`Config::memory`] builds the [`Memory`] client.
//! - [`Ledger`] records, links, bounds, and reads claims.
//! - [`LeanEvidence`] is the port the composition wires to the real Lean
//!   checker; `FakeLeanEvidence` (feature `test-support`) scripts it.
//! - [`tools`] returns `recall_memory`, `ledger_record`, `ledger_claims`,
//!   `ledger_link`, and `ledger_bound` as `supermath-tool` tools.
//!
//! # Example
//!
//! ```
//! use supermath_ledger::{Config, Source, Status};
//!
//! let config = Config::from_lookup(|name| {
//!     (name == "CORTEX_API_KEY").then(|| "key".to_string())
//! })
//! .unwrap();
//! assert_eq!(config.scope_root, "ws:supermath");
//! // A claim from free text can only ever be asserted.
//! assert!(Status::Asserted < Status::Formalised);
//! let _ = Source::Supplied("a remark".into());
//! ```
//!
//! # What this crate does not hold
//!
//! No Lean, no Python, and no literature client: it depends on
//! `supermath-tool` only, and reaches Lean through the [`LeanEvidence`] port
//! and logged calls through [`CallLog`](supermath_tool::CallLog). Wiring the
//! real implementations is `supermath-mcp`'s job, so a promoted claim can be
//! traced to evidence without this crate depending on its producer.

mod config;
mod error;
mod evidence;
mod ledger;
pub mod memory;
mod tools;
mod types;

pub use config::{Config, DEFAULT_CORTEX_URL, DEFAULT_SCOPE_ROOT};
pub use error::{Error, Result};
#[cfg(any(test, feature = "test-support"))]
pub use evidence::FakeLeanEvidence;
pub use evidence::{AXIOM_ALLOWLIST, LeanEvidence, LeanVerdict, LeanVerdictKind};
pub use ledger::{BoundOutcome, BoundRequest, Claim, DEFAULT_WINDOW, Effective, Ledger, Recorded};
pub use memory::{Listed, Memory, Project as MemoryProject, slugify};
pub use tools::{LedgerBound, LedgerClaims, LedgerLink, LedgerRecord, RecallMemory, tools};
pub use types::{Alignment, BoundSpec, Direction, Entry, Kind, LeanClaim, Source, Status};
