//! The tools the agent can call, handed to the `OpenHuman` runner as
//! host tools.
//!
//! Deliberately small for now:
//!
//! | Tool | Exposure | Does |
//! | --- | --- | --- |
//! | `recall_memory` | direct | searches this project's `CortexDB` memory |
//! | `factor_integer` | deferred | exact prime factorization up to 10^12 |
//! | `gcd_lcm` | deferred | exact gcd and lcm of two integers |
//!
//! A deferred tool stays out of the model's initial catalogue. The model finds
//! it through `OpenHuman`'s `tool_search`, which the harness ranks with Jev,
//! so the catalogue can grow without every schema riding every request.
//!
//! Every tool is read-only and takes typed arguments; none evaluates text the
//! model wrote.

mod number;
mod recall;

use openhuman_embed::Tool;

use crate::memory::{Memory, Project};

pub use number::{FACTOR_LIMIT, factorize, gcd};
pub use recall::RECALL_TOOL;

/// The tools for one agent, bound to its project.
#[derive(Debug, Clone)]
pub struct Toolkit {
    memory: Memory,
    project: Project,
}

impl Toolkit {
    /// A toolkit whose memory tool reads `project`'s scope in `memory`.
    #[must_use]
    pub fn new(memory: Memory, project: Project) -> Self {
        Self { memory, project }
    }

    /// Fresh instances of every tool, in catalogue order.
    #[must_use]
    pub fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(recall::RecallTool::new(
                self.memory.clone(),
                self.project.clone(),
            )),
            Box::new(number::FactorTool),
            Box::new(number::GcdTool),
        ]
    }
}

#[cfg(test)]
mod test;
