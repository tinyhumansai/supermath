//! Values the [`Workspace`](super::Workspace) takes and returns.

use crate::path::ProjectPath;

/// A file read from a project, with its digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Where it came from.
    pub path: ProjectPath,
    /// Its UTF-8 text.
    pub text: String,
    /// The lowercase hex SHA-256 of `text`'s bytes.
    pub sha256: String,
}

impl SourceFile {
    /// The number of lines in the file; a trailing newline adds none.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.text.lines().count()
    }
}

/// Who is writing, for the write log, and what they may write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteMeta {
    /// The calling tool, e.g. `file_write`.
    pub tool: String,
    /// The call or turn id.
    pub call_id: String,
    /// Whether the write may land in a generated directory (`Witnesses/`,
    /// `witnesses/`). Only the search tool that owns it sets this.
    pub allow_generated: bool,
}

impl WriteMeta {
    /// Metadata for an ordinary write by `tool` during `call_id`.
    #[must_use]
    pub fn new(tool: impl Into<String>, call_id: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            call_id: call_id.into(),
            allow_generated: false,
        }
    }

    /// Lets the write land in a generated directory.
    #[must_use]
    pub fn allowing_generated(mut self) -> Self {
        self.allow_generated = true;
        self
    }
}

/// What a successful write did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOutcome {
    /// The path written.
    pub path: ProjectPath,
    /// The new digest.
    pub sha256: String,
    /// The digest before, or `None` for a new file.
    pub previous_sha256: Option<String>,
    /// The new line count.
    pub lines: usize,
}

/// One project module a file imports, directly or through others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedModule {
    /// The dotted module name, e.g. `Supermath.Sendov.Lemmas`.
    pub module: String,
    /// The file's digest, or `None` if the module has no file in the project.
    pub sha256: Option<String>,
}

/// The digest over every project module a file imports, transitively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportsDigest {
    /// The lowercase hex SHA-256 over the sorted `module<TAB>digest` lines
    /// (`missing` for a module with no file). With no project imports it is
    /// the digest of the empty string.
    pub sha256: String,
    /// The modules that went into it, sorted by name.
    pub modules: Vec<ImportedModule>,
}
