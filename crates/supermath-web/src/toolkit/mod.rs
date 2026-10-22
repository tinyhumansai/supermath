//! [`WebToolkit`]: the web tools, built from one configuration.

use std::sync::Arc;

use supermath_tool::{Tool, ToolRegistry};

use crate::clock::{Clock, SystemClock};
use crate::config::WebConfig;
use crate::engine::Engine;
use crate::error::Result;
use crate::read::WebRead;
use crate::research::WebResearch;
use crate::search::WebSearch;

/// The three web tools sharing one Gemini client, one daily cap, and one
/// response cache.
#[derive(Debug, Clone)]
pub struct WebToolkit {
    engine: Arc<Engine>,
}

impl WebToolkit {
    /// Builds the toolkit on the system clock.
    ///
    /// # Errors
    ///
    /// [`Error::Client`](crate::Error::Client) if an HTTP client cannot be
    /// built.
    pub fn new(config: WebConfig) -> Result<Self> {
        Self::with_clock(config, Arc::new(SystemClock))
    }

    /// Builds the toolkit on `clock`, which times the cache, the daily cap,
    /// and `retrieved_at`.
    ///
    /// # Errors
    ///
    /// [`Error::Client`](crate::Error::Client) if an HTTP client cannot be
    /// built.
    pub fn with_clock(config: WebConfig, clock: Arc<dyn Clock>) -> Result<Self> {
        Ok(Self {
            engine: Arc::new(Engine::new(config, clock)?),
        })
    }

    /// The `web_search` tool.
    #[must_use]
    pub fn search(&self) -> WebSearch {
        WebSearch::new(Arc::clone(&self.engine))
    }

    /// The `web_research` tool.
    #[must_use]
    pub fn research(&self) -> WebResearch {
        WebResearch::new(Arc::clone(&self.engine))
    }

    /// The `web_read` tool.
    #[must_use]
    pub fn read(&self) -> WebRead {
        WebRead::new(Arc::clone(&self.engine))
    }

    /// Every web tool: `web_search`, `web_research`, `web_read`.
    #[must_use]
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(self.search()),
            Arc::new(self.research()),
            Arc::new(self.read()),
        ]
    }

    /// Adds every web tool to `registry`.
    ///
    /// # Errors
    ///
    /// [`supermath_tool::Error::DuplicateTool`] if a name is already
    /// registered.
    pub fn register(&self, registry: &mut ToolRegistry) -> supermath_tool::Result<()> {
        self.tools()
            .into_iter()
            .try_for_each(|tool| registry.register(tool))
    }
}
