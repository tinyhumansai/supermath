//! Path rules: which files a project workspace may hold.
//!
//! A path is relative to the project root and its first segment names the
//! language:
//!
//! | Language | Pattern |
//! | --- | --- |
//! | Lean | `lean/Supermath/[A-Z][A-Za-z0-9]*(/[A-Z][A-Za-z0-9]*)*.lean` |
//! | Python | `python/work/[a-z][a-z0-9_]*(/[a-z][a-z0-9_]*)*.py` |
//!
//! ```
//! use supermath_files::{Language, ProjectPath};
//!
//! let path = ProjectPath::parse("lean/Supermath/Sendov/Main.lean").unwrap();
//! assert_eq!(path.language(), Language::Lean);
//! assert_eq!(path.module(), "Supermath.Sendov.Main");
//! assert!(ProjectPath::parse("lean/Supermath/../x.lean").is_err());
//! ```

mod types;

pub use types::{Language, MAX_PATH_BYTES, ProjectPath};

#[cfg(test)]
mod test;
