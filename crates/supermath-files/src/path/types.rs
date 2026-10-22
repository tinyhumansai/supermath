//! [`Language`] and [`ProjectPath`].

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The two languages a project workspace holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    /// A Lean 4 file under `lean/Supermath/`.
    Lean,
    /// A Python file under `python/work/`.
    Python,
}

impl Language {
    /// The first path segment of this language's files: `lean` or `python`.
    #[must_use]
    pub fn root_dir(self) -> &'static str {
        match self {
            Self::Lean => "lean",
            Self::Python => "python",
        }
    }

    /// The file extension, without the dot.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Lean => "lean",
            Self::Python => "py",
        }
    }

    /// The package directory under the root: `Supermath` or `work`.
    #[must_use]
    pub fn package_dir(self) -> &'static str {
        match self {
            Self::Lean => "Supermath",
            Self::Python => "work",
        }
    }

    /// The directory name only the owning search tool writes into.
    #[must_use]
    pub fn generated_dir(self) -> &'static str {
        match self {
            Self::Lean => "Witnesses",
            Self::Python => "witnesses",
        }
    }

    fn segment_ok(self, segment: &str) -> bool {
        let mut bytes = segment.bytes();
        match self {
            Self::Lean => {
                bytes.next().is_some_and(|b| b.is_ascii_uppercase())
                    && bytes.all(|b| b.is_ascii_alphanumeric())
            }
            Self::Python => {
                bytes.next().is_some_and(|b| b.is_ascii_lowercase())
                    && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            }
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.root_dir())
    }
}

/// The longest path accepted, in bytes.
pub const MAX_PATH_BYTES: usize = 256;

/// A path inside a project's workspace that satisfies the path rules.
///
/// Holding one proves the string is relative, has no `..`, names a language
/// root, and matches that language's module-name pattern. It says nothing
/// about the file existing or about symlinks; [`Workspace`](crate::Workspace)
/// checks those.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectPath {
    language: Language,
    raw: String,
}

impl ProjectPath {
    /// Parses and validates `raw`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidPath`] naming the first rule broken.
    pub fn parse(raw: &str) -> Result<Self> {
        let fail = |reason| Error::InvalidPath {
            path: raw.to_owned(),
            reason,
        };
        if raw.is_empty() {
            return Err(fail("path is empty"));
        }
        if raw.len() > MAX_PATH_BYTES {
            return Err(fail("path is too long"));
        }
        if raw.starts_with('/') || raw.contains('\\') || raw.contains('\0') || raw.contains(':') {
            return Err(fail("path must be relative to the project root"));
        }
        let segments: Vec<&str> = raw.split('/').collect();
        if segments
            .iter()
            .any(|s| s.is_empty() || *s == "." || *s == "..")
        {
            return Err(fail("path has an empty, `.`, or `..` segment"));
        }
        let language = match segments[0] {
            "lean" => Language::Lean,
            "python" => Language::Python,
            _ => return Err(fail("path must start with lean/ or python/")),
        };
        if segments.get(1) != Some(&language.package_dir()) || segments.len() < 3 {
            return Err(fail("path must be a module under the language's package"));
        }
        let last = segments.len() - 1;
        let stem = segments[last]
            .strip_suffix(&format!(".{}", language.extension()))
            .ok_or_else(|| fail("file name must end with the language's extension"))?;
        let dirs = &segments[2..last];
        if !dirs.iter().chain([&stem]).all(|s| language.segment_ok(s)) {
            return Err(fail("a segment is not a valid module name"));
        }
        Ok(Self {
            language,
            raw: raw.to_owned(),
        })
    }

    /// The file for a dotted module name such as `Supermath.Sendov.Main` or
    /// `work.sendov_search`, or `None` if the name is not a module of the
    /// language's package.
    #[must_use]
    pub fn from_module(language: Language, module: &str) -> Option<Self> {
        let mut parts = module.split('.');
        if parts.next() != Some(language.package_dir()) {
            return None;
        }
        let rest: Vec<&str> = parts.collect();
        let raw = format!(
            "{}/{}/{}.{}",
            language.root_dir(),
            language.package_dir(),
            rest.join("/"),
            language.extension()
        );
        Self::parse(&raw).ok()
    }

    /// The path as a string, e.g. `lean/Supermath/Sendov/Main.lean`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The path's language.
    #[must_use]
    pub fn language(&self) -> Language {
        self.language
    }

    /// The dotted module name: `Supermath.Sendov.Main` or `work.sendov_search`.
    #[must_use]
    pub fn module(&self) -> String {
        let without_ext = &self.raw[..self.raw.len() - self.language.extension().len() - 1];
        without_ext.split('/').skip(1).collect::<Vec<_>>().join(".")
    }

    /// Whether the file sits in a generated directory (`Witnesses/` or
    /// `witnesses/`), which only the tool that owns it writes.
    #[must_use]
    pub fn is_generated(&self) -> bool {
        let segments: Vec<&str> = self.raw.split('/').collect();
        segments[..segments.len() - 1].contains(&self.language.generated_dir())
    }
}

impl fmt::Display for ProjectPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}
