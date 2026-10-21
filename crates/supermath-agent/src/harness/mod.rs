//! The solving harness: one `OpenHuman` agent turn per problem, wrapped in
//! `CortexDB` recall before and a write-back after.
//!
//! [`Harness::start`] boots the embedded `OpenHuman` runtime once per
//! process, wired to two endpoints:
//!
//! - the **LLM ladder** is the runtime's provider, so every turn runs on a
//!   ladder such as `reasoning`;
//! - **Jev**, reached through `OpenRouter`, ranks `tool_search`, with the
//!   ladder's `vectors` model as its retriever.
//!
//! [`Harness::solve`] then, for one [`Problem`]:
//!
//! 1. recalls what memory holds for the problem;
//! 2. registers a solver agent whose only tools are the [`Toolkit`], bound to
//!    this problem, and runs one turn;
//! 3. reads the [`Verdict`] from the reply;
//! 4. writes the verdict and the reply back to memory.
//!
//! A memory failure does not fail the run. It is carried on the [`Report`]
//! instead, so the caller sees both the verdict and that memory was not
//! updated; it is never swallowed.

mod prompt;

use std::sync::Arc;

use openhuman_embed::{
    AgentDefinitionSpec, AgentSpec, HostTurnTools, Provider, Runtime, RuntimeConfig,
    RuntimeEventKind, RuntimeEvents, ToolScopeSpec, Workspace,
};
use openhuman_tinyhumans::jev::TinyHumansJevRanker;
use tinyinference_embeddings::OpenAiEmbeddingModel;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::memory::Memory;
use crate::problem::Problem;
use crate::toolkit::Toolkit;
use crate::verdict::Verdict;

/// The most tool-loop iterations one solving turn may take.
const MAX_ITERATIONS: usize = 24;

/// How long to wait for a turn's last events after the turn returns. The
/// core bus delivers them asynchronously.
const EVENT_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

/// One tool call the solver made, without its arguments or result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    /// The declared tool name, such as `tool_search` or `factor_integer`.
    pub name: String,
    /// Whether the call succeeded.
    pub success: bool,
}

/// What one run produced.
#[derive(Debug, Clone)]
pub struct Report {
    /// What the run established.
    pub verdict: Verdict,
    /// The agent's full reply.
    pub reply: String,
    /// Every tool call the turn made, in order. The evidence for what the
    /// reply says it computed.
    pub tool_calls: Vec<ToolCall>,
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

/// The `OpenHuman` config this harness runs under.
fn runtime_config() -> RuntimeConfig {
    let mut config = RuntimeConfig::default();
    // No local model runtime: every inference goes to the ladder.
    config.local_ai.runtime_enabled = false;
    // Jev decides `tool_search`, through `OpenRouter` and nothing else. The
    // route is an operator decision, so it is named rather than left `auto`.
    config.agent.tool_search.ranker = "jev".into();
    config.agent.tool_search.jev_route = "openrouter".into();
    config
}

/// The Jev ranker: `OpenRouter` for decisions, the ladder for retrieval.
fn jev_ranker(config: &Config, runtime_config: RuntimeConfig) -> Arc<TinyHumansJevRanker> {
    let openrouter_key = config.openrouter_api_key.clone();
    let embedder = OpenAiEmbeddingModel::new(config.ladder_api_key.clone())
        .with_base_url(config.ladder_url.clone())
        .with_model(config.embedding_model.clone())
        .with_dimensions(config.embedding_dims);
    let embedder: Arc<dyn tinyinference_embeddings::EmbeddingModel> = Arc::new(embedder);
    Arc::new(
        TinyHumansJevRanker::new()
            // The ranker otherwise reads the process config from disk, which
            // would not carry the route set above.
            .with_config_loader(Arc::new(move || {
                let config = runtime_config.clone();
                Box::pin(async move { Ok(config) })
            }))
            .with_env_loader(Arc::new(move |name| {
                (name == "OPENROUTER_API_KEY").then(|| openrouter_key.clone())
            }))
            .with_embedder_factory(Arc::new(move |_, _| embedder.clone())),
    )
}

impl Harness {
    /// Boots the `OpenHuman` runtime described by `config`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Runtime`] when the runtime fails to build.
    pub async fn start(config: &Config) -> Result<Self> {
        let runtime_config = runtime_config();
        let provider = Provider::openai_compatible(&config.ladder_url, &config.ladder_api_key)
            .model(&config.model);
        let mut builder = Runtime::builder()
            .config(runtime_config.clone())
            .workspace(Workspace::Dir(config.workspace.clone()))
            .provider(provider)
            .tool_ranker(jev_ranker(config, runtime_config));
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

        let agent_id = format!("solver-{}", problem.slug());
        // Subscribed before the agent exists, so no event of its turn is missed.
        let collector = tokio::spawn(collect_tool_calls(self.runtime.events(), agent_id.clone()));

        let toolkit = Toolkit::new(self.memory.clone(), problem.clone());
        let agent = self
            .runtime
            .agent(
                AgentSpec::new(agent_id)
                    .definition(
                        AgentDefinitionSpec::new()
                            .bare_prompt(prompt::SOLVER_PROMPT)
                            .tools(ToolScopeSpec::HostOnly)
                            .max_iterations(MAX_ITERATIONS),
                    )
                    .tools(move |_| HostTurnTools::advertised(toolkit.tools())),
            )
            .map_err(|error| Error::Runtime(error.to_string()))?;

        let outcome = agent
            .run(&prompt::turn_message(problem, recalled.as_deref()))
            .await
            .map_err(|error| Error::Turn(error.to_string()))?;
        let verdict = Verdict::parse(&outcome.reply);
        let tool_calls = match tokio::time::timeout(EVENT_GRACE, collector).await {
            Ok(Ok(calls)) => calls,
            // The turn ended without its closing event arriving in time; what
            // was seen so far is lost with the task, which is reported as none.
            _ => Vec::new(),
        };

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
            tool_calls,
            recall_error,
            memory_event,
            memory_error,
        })
    }
}

/// Records `agent_id`'s tool calls until its turn ends.
async fn collect_tool_calls(mut events: RuntimeEvents, agent_id: String) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    while let Ok(event) = events.recv().await {
        if event.agent_id.as_deref() != Some(agent_id.as_str()) {
            continue;
        }
        match event.kind {
            RuntimeEventKind::ToolEnded { tool_name, success } => calls.push(ToolCall {
                name: tool_name,
                success,
            }),
            RuntimeEventKind::TurnEnded { .. } => break,
            _ => {}
        }
    }
    calls
}

#[cfg(test)]
mod test;
