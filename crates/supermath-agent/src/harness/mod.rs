//! The solving harness: one `OpenHuman` agent turn per problem, wrapped in
//! `CortexDB` recall before and a write-back after.
//!
//! [`Harness::start`] boots the embedded `OpenHuman` runtime once per
//! process. [`Harness::solve`] then, for one [`Problem`]:
//!
//! 1. recalls what memory holds for the problem;
//! 2. registers a solver agent that sees only the `recall_memory` host tool,
//!    bound to this problem's scope, and runs one turn;
//! 3. reads the [`Verdict`] from the reply;
//! 4. writes the verdict and the reply back to memory.
//!
//! A memory failure does not fail the run. It is carried on the [`Report`]
//! instead, so the caller sees both the verdict and that memory was not
//! updated; it is never swallowed.

mod prompt;
mod tool;

use openhuman_embed::{
    AgentDefinitionSpec, AgentSpec, HostTurnTools, Provider, Runtime, RuntimeConfig, ToolScopeSpec,
    Workspace,
};

use crate::config::Config;
use crate::error::{Error, Result};
use crate::memory::Memory;
use crate::problem::Problem;
use crate::verdict::Verdict;

use tool::RecallTool;

/// The most tool-loop iterations one solving turn may take.
const MAX_ITERATIONS: usize = 24;

/// What one run produced.
#[derive(Debug, Clone)]
pub struct Report {
    /// What the run established.
    pub verdict: Verdict,
    /// The agent's full reply.
    pub reply: String,
    /// Why recall before the turn failed, when it did.
    pub recall_error: Option<String>,
    /// The memory event the run was stored as, when the write succeeded.
    pub memory_event: Option<String>,
    /// Why the write-back failed, when it did.
    pub memory_error: Option<String>,
}

/// The embedded runtime and the memory client every run shares.
#[derive(Debug)]
pub struct Harness {
    runtime: Runtime,
    memory: Memory,
}

impl Harness {
    /// Boots the `OpenHuman` runtime described by `config`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Runtime`] when the runtime fails to build.
    pub async fn start(config: &Config) -> Result<Self> {
        let mut runtime_config = RuntimeConfig::default();
        // No local model runtime: every inference goes to the configured
        // OpenAI-compatible route.
        runtime_config.local_ai.runtime_enabled = false;
        let provider =
            Provider::openai_compatible(&config.api_base_url, &config.api_key).model(&config.model);
        let mut builder = Runtime::builder()
            .config(runtime_config)
            .workspace(Workspace::Dir(config.workspace.clone()))
            .provider(provider);
        if let Some(url) = &config.backend_url {
            builder = builder.backend_url(url);
        }
        // The build future is large; boxing it keeps it off the caller's stack.
        let runtime = Box::pin(builder.build())
            .await
            .map_err(|error| Error::Runtime(error.to_string()))?;
        let memory = Memory::new(
            &config.cortex_url,
            &config.cortex_api_key,
            &config.scope_root,
        );
        Ok(Self { runtime, memory })
    }

    /// Works on `problem` for one agent turn and reports the verdict.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Runtime`] when the solver agent cannot be registered,
    /// and [`Error::Turn`] when the turn itself fails. Memory failures are
    /// reported on the [`Report`], not as errors.
    pub async fn solve(&self, problem: &Problem) -> Result<Report> {
        let (recalled, recall_error) = match self.memory.recall(problem, problem.statement()).await
        {
            Ok(notes) => (notes, None),
            Err(error) => (None, Some(error.to_string())),
        };

        let recall_tool = RecallTool::new(self.memory.clone(), problem.clone());
        let agent = self
            .runtime
            .agent(
                AgentSpec::new(format!("solver-{}", problem.slug()))
                    .definition(
                        AgentDefinitionSpec::new()
                            .bare_prompt(prompt::SOLVER_PROMPT)
                            .tools(ToolScopeSpec::HostOnly)
                            .max_iterations(MAX_ITERATIONS),
                    )
                    .tools(move |_| HostTurnTools::advertised(vec![Box::new(recall_tool.clone())])),
            )
            .map_err(|error| Error::Runtime(error.to_string()))?;

        let outcome = agent
            .run(&prompt::turn_message(problem, recalled.as_deref()))
            .await
            .map_err(|error| Error::Turn(error.to_string()))?;
        let verdict = Verdict::parse(&outcome.reply);

        let note = prompt::memory_note(&verdict, &outcome.reply);
        let label = format!("verdict:{}", verdict.label());
        let (memory_event, memory_error) = match self
            .memory
            .remember(problem, &note, &[label.as_str()])
            .await
        {
            Ok(event) => (Some(event), None),
            Err(error) => (None, Some(error.to_string())),
        };

        Ok(Report {
            verdict,
            reply: outcome.reply,
            recall_error,
            memory_event,
            memory_error,
        })
    }
}

#[cfg(test)]
mod test;
