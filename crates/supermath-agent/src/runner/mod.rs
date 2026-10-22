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
//! [`Runner::agent`] composes the toolkit for one project, builds the
//! `agent_*` children on the same runtime, and registers the top-level agent:
//! a `Named` tool scope listing the adapted toolkit, the `agent_*` tools, and
//! `tool_search`, with `OpenHuman`'s own `memory` tool disallowed so it does
//! not shadow the ledger's `recall_memory`. Jacobian and lean-lsp-mcp join as
//! stdio MCP servers when their commands are configured.

mod types;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use openhuman_embed::{
    AgentDefinitionSpec, AgentSpec, HostTurnTools, McpServer, Provider, Runtime, RuntimeConfig,
    ToolScopeSpec, Workspace,
};
use openhuman_tinyhumans::jev::{OPENROUTER_API_KEY_ENV, TinyHumansJevRanker};
use supermath_mcp::Toolkit;
use supermath_subagents::{Subagents, SubagentsConfig};
use tinyinference_embeddings::OpenAiEmbeddingModel;

pub use types::Solver;

use crate::config::Config;

use crate::error::{Error, Result};
use crate::host::Host;

/// The most tool-loop iterations one turn may take.
pub const MAX_ITERATIONS: usize = 24;

/// The agent's system prompt. A placeholder until the agent's design lands.
pub const SYSTEM_PROMPT: &str = "\
You are supermath, a careful mathematical assistant.
Use `recall_memory` to check what earlier work on this project recorded, and
ask `tool_search` for tools before computing anything exact by hand.
Hand long jobs (reading a paper, formalizing a statement, hunting a
counterexample) to the `agent_*` tools, and quote the `call_id` of the tool
call that backs a claim when you record it.";

/// The tool `OpenHuman` ranks deferred tools with; the scope must list it.
const TOOL_SEARCH: &str = "tool_search";

/// `OpenHuman`'s own memory tool, which would shadow the ledger's.
const OPENHUMAN_MEMORY: &str = "memory";

/// The slug Jacobian's tools appear under, as `mcp_jacobian_<tool>`.
const JACOBIAN_SERVER: &str = "jacobian";

/// The slug lean-lsp-mcp's tools appear under, as `mcp_lean_lsp_<tool>`.
const LEAN_LSP_SERVER: &str = "lean_lsp";

/// The embedded runtime every agent of a process shares.
#[derive(Debug)]
pub struct Runner {
    runtime: Arc<Runtime>,
    config: Config,
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
    let embedder = OpenAiEmbeddingModel::new(config.surplus_api_key.clone())
        .with_base_url(config.surplus_url.clone())
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
        let provider = Provider::openai_compatible(&config.surplus_url, &config.surplus_api_key)
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
        Ok(Self {
            runtime: Arc::new(runtime),
            config: config.clone(),
        })
    }

    /// Composes the toolkit for `project`, builds the `agent_*` children, and
    /// registers the top-level agent.
    ///
    /// # Errors
    ///
    /// [`Error::Toolkit`] when `project` has no usable characters or a domain
    /// cannot build; [`Error::Subagents`] when `OpenHuman` refuses a child;
    /// [`Error::DuplicateTool`] when two tools share a name;
    /// [`Error::Runtime`] when `OpenHuman` refuses the agent.
    pub fn agent(&self, project: &str) -> Result<Solver> {
        let toolkit = Arc::new(Toolkit::compose(&self.config.toolkit, project)?);
        let subagents = Subagents::new(
            &self.runtime,
            &toolkit.registry(),
            toolkit.call_log(),
            toolkit.project(),
            &SubagentsConfig {
                models: self.config.models.clone(),
                ..SubagentsConfig::default()
            },
        )?;
        let host = Arc::new(Host::new(Arc::clone(&toolkit), subagents.tools())?);

        let mut scope: Vec<String> = host.tool_names().into_iter().map(str::to_owned).collect();
        scope.push(TOOL_SEARCH.to_owned());
        let belt = Arc::clone(&host);
        let mut spec = AgentSpec::new(format!("supermath-{}", toolkit.project().slug()))
            .definition(
                AgentDefinitionSpec::new()
                    .bare_prompt(SYSTEM_PROMPT)
                    .tools(ToolScopeSpec::Named(scope.clone()))
                    .disallow_tools([OPENHUMAN_MEMORY])
                    .max_iterations(MAX_ITERATIONS),
            )
            .tools(move |_| HostTurnTools::advertised(belt.belt()));
        for server in mcp_servers(&self.config) {
            spec = spec.mcp(server);
        }
        let agent = self
            .runtime
            .agent(spec)
            .map_err(|error| Error::Runtime(error.to_string()))?;
        Ok(Solver {
            agent,
            host,
            scope,
            unavailable: subagents
                .unavailable()
                .iter()
                .map(|gap| gap.tool.to_owned())
                .collect(),
        })
    }
}

/// The external MCP servers `config` names, in a fixed order.
fn mcp_servers(config: &Config) -> Vec<McpServer> {
    [
        (JACOBIAN_SERVER, &config.jacobian_cmd),
        (LEAN_LSP_SERVER, &config.lean_lsp_cmd),
    ]
    .into_iter()
    .filter_map(|(name, command)| stdio_server(name, command.as_deref()?))
    .collect()
}

/// A stdio server for a command line: the first word runs, the rest are its
/// arguments. `None` for a blank line.
fn stdio_server(name: &str, command_line: &str) -> Option<McpServer> {
    let mut words = command_line.split_whitespace();
    let command = words.next()?;
    Some(McpServer::stdio(name, command, words))
}

#[cfg(test)]
mod test;
