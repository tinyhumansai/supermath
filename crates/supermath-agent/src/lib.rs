//! The supermath agent scaffold: the embedded `OpenHuman` runtime, wired to
//! the LLM ladder, Jev, a small toolkit, and `CortexDB` memory.
//!
//! This crate sets up the pieces an agent runs on. What the agent does with
//! them is not designed yet.
//!
//! # Entry points
//!
//! - [`Config::from_env`] reads the ladder, Jev, and memory settings.
//! - [`Runner::start`] boots `OpenHuman`; [`Runner::agent`] registers the
//!   agent for a [`Project`], with the [`Toolkit`] as its tools.
//! - [`Memory`] is the `CortexDB` client: scoped recall, indexed writes, and
//!   a readiness check.
//!
//! # Example
//!
//! ```
//! use supermath_agent::{Memory, Project};
//!
//! let project = Project::new("Twin Primes")?;
//! assert_eq!(project.slug(), "twin-primes");
//!
//! let memory = Memory::new("http://localhost:3141", "key", "ws:supermath");
//! assert_eq!(memory.scope(&project), "ws:supermath/project:twin-primes/app:brain");
//! # Ok::<(), supermath_agent::Error>(())
//! ```
//!
//! # What this crate does not hold
//!
//! No agent loop, tool runtime, or provider client of its own: those are
//! `OpenHuman`'s, and model routing is the ladder's.

mod config;
mod error;
mod memory;
mod runner;
mod toolkit;

pub use config::{
    Config, DEFAULT_CORTEX_URL, DEFAULT_EMBEDDING_DIMS, DEFAULT_EMBEDDING_MODEL,
    DEFAULT_LADDER_URL, DEFAULT_MODEL, DEFAULT_REASONING_EFFORT, DEFAULT_SCOPE_ROOT,
    DEFAULT_WORKSPACE, REASONING_EFFORTS,
};
pub use error::{Error, Result};
pub use memory::{Memory, Project, WRITE_BARRIER, reached_barrier, render_pack, slugify};
pub use runner::{MAX_ITERATIONS, Runner, SYSTEM_PROMPT};
pub use toolkit::{FACTOR_LIMIT, RECALL_TOOL, Toolkit, factorize, gcd};
