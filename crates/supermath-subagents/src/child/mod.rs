//! The four children: what each is, what it may call, and how a turn runs.
//!
//! A child is a separate top-level `OpenHuman` `Agent`, built ahead of time
//! with a bare prompt, a `Named` tool scope, and the primitives of its
//! [`ChildSpec`] as host tools. `OpenHuman`'s built-in sub-agents never see
//! host tools, which is why the children are not `delegate_<id>` workers.
//!
//! [`ChildRunner`] is the port an `agent_*` tool drives. [`OpenHumanChild`]
//! is the real implementation, one `Agent::turn` per call; tests substitute a
//! runner that plays a fixed sequence of calls against fake primitives.

mod agent;
mod prompts;
mod types;

pub use agent::OpenHumanChild;
pub use types::{ChildEnd, ChildKind, ChildRequest, ChildRunner, ChildSpec, ChildTurn};

#[cfg(test)]
mod test;
