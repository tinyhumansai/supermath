//! `supermath-mcp` composes the supermath tool domains into one [`Toolkit`]
//! for a project, wires each domain's ports to the others, and serves the
//! result over MCP.
//!
//! The crate owns no tools. [`Toolkit::compose`] builds every primitive
//! domain from a [`Config`] and registers their tools in one registry;
//! [`Toolkit::dispatch`] runs a call and records it in the call log that the
//! ledger reads when it validates a `tool` source. [`serve`] adapts the
//! toolkit to the Model Context Protocol on standard input and output, and
//! the `supermath-mcp` binary wraps it:
//!
//! ```text
//! claude mcp add supermath -- supermath-mcp serve my-project
//! ```
//!
//! ```
//! use serde_json::json;
//! use supermath_mcp::{Config, Toolkit};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> supermath_mcp::Result<()> {
//! // Nothing configured: Lean, Python, Gemini and CortexDB are absent, yet
//! // every tool is registered and calc_factor works.
//! let config = Config::from_lookup(|_| None)?;
//! let toolkit = Toolkit::compose(&config, "my-project")?;
//! let done = toolkit.dispatch("calc_factor", json!({"n": 360})).await;
//! assert!(done.envelope.is_ok());
//! assert_eq!(done.call_id, "call-1");
//! # Ok(()) }
//! ```
//!
//! # What this crate does not hold
//!
//! - **Tools.** Each lives in its domain crate. This crate only composes
//!   them.
//! - **Agents.** No tool runs a model turn of its own; the seats of an
//!   `rsi-ralph` loop are the agents, and they call these tools.
//! - **A claim's status.** Only the ledger sets one; [`LeanBridge`] hands it
//!   a verdict and nothing more.
//! - **The project.** It is fixed at composition, from the command line.

pub mod check;
mod claims;
pub mod cli;
mod config;
mod error;
mod evidence;
mod server;
mod toolkit;

pub use claims::{RecordedClaims, RecordsClaims};
pub use config::Config;
pub use error::{Error, Result};
pub use evidence::LeanBridge;
pub use server::{McpServer, serve, serve_on};
pub use toolkit::{Dispatched, Toolkit};
