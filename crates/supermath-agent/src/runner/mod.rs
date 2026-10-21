//! The `OpenHuman` runner: the embedded runtime, wired to the ladder, Jev, and
//! the toolkit.
//!
//! [`Runner::start`] boots the runtime once per process:
//!
//! - the **LLM ladder** is the provider, so every turn runs on a ladder such
//!   as `flash`, at the configured reasoning effort;
//! - **Jev**, reached through `OpenRouter`, ranks `tool_search` over deferred
//!   tools, with the ladder's `vectors` model as its retriever.
//!
//! [`Runner::agent`] registers an agent for one [`Project`] whose only tools
//! are the [`Toolkit`], bound to that project's memory. What the agent is
//! asked to do is not decided here yet.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use openhuman_embed::{
    Agent, AgentDefinitionSpec, AgentSpec, HostTurnTools, Provider, Runtime, RuntimeConfig,
    ToolScopeSpec, Workspace,
};
use openhuman_tinyhumans::jev::{OPENROUTER_API_KEY_ENV, TinyHumansJevRanker};
use tinyinference_embeddings::OpenAiEmbeddingModel;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::memory::{Memory, Project};
use crate::toolkit::Toolkit;

/// The most tool-loop iterations one turn may take.
pub const MAX_ITERATIONS: usize = 24;

/// The agent's system prompt. A placeholder until the agent's design lands.
pub const SYSTEM_PROMPT: &str = "\
You are supermath, a careful mathematical assistant.
Use `recall_memory` to check what earlier work on this project recorded, and
ask `tool_search` for tools before computing anything exact by hand.";

/// The embedded runtime and the memory client every agent shares.
#[derive(Debug)]
pub struct Runner {
    runtime: Runtime,
    memory: Memory,
}

/// The `OpenHuman` config the runner runs under.
fn runtime_config(settings: &Config) -> RuntimeConfig {
    let mut config = RuntimeConfig::default();
    // No local model runtime: every inference goes to the ladder.
    config.local_ai.runtime_enabled = false;
    // Every turn asks for this depth; the ladder passes it to the rung.
    config.runtime.reasoning_effort = Some(settings.reasoning_effort.clone());
    // Jev decides `tool_search`, through `OpenRouter` and nothing else. The
    // route is an operator decision, so it is named rather than left `auto`.
    config.agent.tool_search.ranker = "jev".into();
    config.agent.tool_search.jev_route = "openrouter".into();
    config
}

/// The Jev ranker: `OpenRouter` for decisions, the ladder for retrieval.
fn jev_ranker(config: &Config, mut runtime_config: RuntimeConfig) -> Arc<TinyHumansJevRanker> {
    // The same paths `Workspace::Dir` gives the runtime. Left at the defaults,
    // the ranker's embedding cache would open the operator's own
    // `~/.openhuman` storage.
    runtime_config.workspace_dir.clone_from(&config.workspace);
    runtime_config.config_path = config
        .workspace
        .parent()
        .unwrap_or(&config.workspace)
        .join("config.toml");
    let embedder = OpenAiEmbeddingModel::new(config.ladder_api_key.clone())
        .with_base_url(config.ladder_url.clone())
        .with_model(config.embedding_model.clone())
        .with_dimensions(config.embedding_dims);
    let embedder: Arc<dyn tinyinference_embeddings::EmbeddingModel> = Arc::new(embedder);
    Arc::new(
        TinyHumansJevRanker::new()
            .with_config_loader(fixed_config(runtime_config))
            .with_env_loader(jev_credential(config.openrouter_api_key.clone()))
            .with_embedder_factory(Arc::new(move |_, _| embedder.clone())),
    )
}

/// The ranker's config loader, spelled as `TinyHumansJevRanker` takes it.
type ConfigLoader = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = std::result::Result<RuntimeConfig, String>> + Send>>
        + Send
        + Sync,
>;

/// The ranker's environment lookup, spelled as `TinyHumansJevRanker` takes it.
type EnvLoader = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// A config loader that always answers `config`.
///
/// The ranker otherwise reads the process config from disk, which would not
/// carry the Jev route this runner sets.
fn fixed_config(config: RuntimeConfig) -> ConfigLoader {
    Arc::new(move || {
        let config = config.clone();
        Box::pin(async move { Ok(config) })
    })
}

/// An environment that holds the `OpenRouter` key and nothing else, so the
/// ranker never reads Jev credentials from the process environment.
fn jev_credential(openrouter_key: String) -> EnvLoader {
    Arc::new(move |name| (name == OPENROUTER_API_KEY_ENV).then(|| openrouter_key.clone()))
}

impl Runner {
    /// Boots the `OpenHuman` runtime described by `config`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Runtime`] when the runtime fails to build.
    pub async fn start(config: &Config) -> Result<Self> {
        let runtime_config = runtime_config(config);
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

    /// The memory client every agent's tools share.
    #[must_use]
    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// Registers the agent for `project`, with the [`Toolkit`] as its tools.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Runtime`] when `OpenHuman` refuses the agent.
    pub fn agent(&self, project: &Project) -> Result<Agent> {
        let toolkit = Toolkit::new(self.memory.clone(), project.clone());
        self.runtime
            .agent(
                AgentSpec::new(format!("supermath-{}", project.slug()))
                    .definition(
                        AgentDefinitionSpec::new()
                            .bare_prompt(SYSTEM_PROMPT)
                            .tools(ToolScopeSpec::HostOnly)
                            .max_iterations(MAX_ITERATIONS),
                    )
                    .tools(move |_| HostTurnTools::advertised(toolkit.tools())),
            )
            .map_err(|error| Error::Runtime(error.to_string()))
    }
}

#[cfg(test)]
mod test;
