//! The crate-wide [`Error`] and [`Result`].
//!
//! Each domain crate keeps its own error enum; this one wraps the ones
//! composition can meet (a domain refusing to build, the contract refusing a
//! duplicate tool) and adds the failures only this crate has: a project name
//! that is not a slug, an unreadable `--env-file`, a command line it does not
//! understand, and the MCP transport failing.

/// What can go wrong composing, serving, or checking the toolkit.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The project name has no usable characters.
    #[error("invalid project name {0:?}")]
    InvalidProject(String),
    /// The tool contract refused the composition, for example two domains
    /// registering the same tool name.
    #[error("tool registry: {0}")]
    Tool(#[from] supermath_tool::Error),
    /// The web domain could not be configured or built.
    #[error("web: {0}")]
    Web(#[from] supermath_web::Error),
    /// The ledger could not be configured or built.
    #[error("ledger: {0}")]
    Ledger(#[from] supermath_ledger::Error),
    /// The Lean domain could not be built.
    #[error("lean: {0}")]
    Lean(#[from] supermath_lean::Error),
    /// The Python domain could not be built.
    #[error("python: {0}")]
    Python(#[from] supermath_python::Error),
    /// The literature domain could not be built.
    #[error("literature: {0}")]
    Literature(#[from] supermath_literature::Error),
    /// An `--env-file` could not be read.
    #[error("env file {path}: {source}")]
    EnvFile {
        /// The file as given.
        path: String,
        /// Why it could not be read.
        #[source]
        source: std::io::Error,
    },
    /// `loop new` was pointed at a folder that already exists.
    #[error("{0} already exists")]
    LoopExists(String),
    /// A loop's statement file does not declare the conjecture.
    #[error("invalid statement: {0}")]
    InvalidStatement(String),
    /// The compiled-in loop template is not valid JSON where it must be.
    #[error("loop template: {0}")]
    Template(String),
    /// A scorer ran without the variables rsi-ralph sets for it.
    #[error("{0} is not set; run this as an rsi-ralph scorer")]
    MissingEnv(&'static str),
    /// A file of a loop folder or a scorer's working copy could not be
    /// read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file or folder.
        path: String,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The command line was not understood.
    #[error("{0}")]
    Usage(String),
    /// The MCP transport failed: the handshake never completed, or the
    /// connection broke.
    #[error("mcp transport: {0}")]
    Transport(String),
}

/// The crate's standard result type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test;
