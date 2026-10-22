//! Import rules: which modules a file may name.
//!
//! | Language | May import |
//! | --- | --- |
//! | Lean | `Mathlib`, `Mathlib.*`, and `Supermath.*` of this project |
//! | Python | the standard library, the solver environment, and `work`/`work.*` of this project |
//!
//! The check is a parse of the import header (Lean) or of every `import` and
//! `from … import` statement (Python), not a sandbox: the language services
//! are the real boundary, and this gives the caller an early, specific
//! refusal. Python relative imports are refused; write `work.x` instead.
//!
//! ```
//! use supermath_files::{Language, imports};
//!
//! assert!(imports::check(Language::Lean, "import Mathlib\nimport Supermath.A").is_ok());
//! assert!(imports::check(Language::Lean, "import Lean").is_err());
//! assert!(imports::check(Language::Python, "import numpy\nfrom work import a").is_ok());
//! assert!(imports::check(Language::Python, "import requests").is_err());
//! ```

mod lean;
mod python;
mod stdlib;

use crate::error::{Error, Result};
use crate::path::Language;

pub(crate) use lean::strip_comments as strip_lean_comments;

/// One import statement: the module and, for `from m import a, b`, the names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The module as written, e.g. `Mathlib.Tactic` or `work.a`.
    pub module: String,
    /// For a Python `from` import, the imported names; otherwise empty.
    pub names: Vec<String>,
}

/// Every import `source` declares, in order, whether allowed or not.
#[must_use]
pub fn parse(language: Language, source: &str) -> Vec<Import> {
    match language {
        Language::Lean => lean::header_imports(source)
            .into_iter()
            .map(|module| Import {
                module,
                names: Vec::new(),
            })
            .collect(),
        Language::Python => python::statements(source),
    }
}

/// Whether `module` belongs to this project (`Supermath.*`, `work`, `work.*`)
/// rather than to the toolchain.
#[must_use]
pub fn is_project_module(language: Language, module: &str) -> bool {
    let package = language.package_dir();
    match language {
        Language::Lean => module.starts_with("Supermath."),
        Language::Python => module == package || module.starts_with("work."),
    }
}

/// Checks `source` against the import rules and returns its imports.
///
/// # Errors
///
/// [`Error::ForbiddenImport`] for the first import outside the table above.
pub fn check(language: Language, source: &str) -> Result<Vec<Import>> {
    let imports = parse(language, source);
    for import in &imports {
        if let Some(reason) = refusal(language, &import.module) {
            return Err(Error::ForbiddenImport {
                language,
                module: import.module.clone(),
                reason,
            });
        }
    }
    Ok(imports)
}

fn refusal(language: Language, module: &str) -> Option<&'static str> {
    match language {
        Language::Lean => {
            let mathlib = module == "Mathlib" || module.starts_with("Mathlib.");
            (!mathlib && !is_project_module(language, module))
                .then_some("only Mathlib and Supermath.* may be imported")
        }
        Language::Python => {
            if module.starts_with('.') {
                return Some("relative imports are not allowed; import work.<module>");
            }
            let top = module.split('.').next().unwrap_or_default();
            let allowed = is_project_module(language, module)
                || stdlib::STDLIB.contains(&top)
                || stdlib::SOLVER_ENVIRONMENT.contains(&top);
            (!allowed).then_some(
                "only the standard library, the solver environment, and work.* may be imported",
            )
        }
    }
}

#[cfg(test)]
mod test;
