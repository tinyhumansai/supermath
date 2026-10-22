//! The provenance digest of a file's project imports.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;

use super::types::{ImportedModule, ImportsDigest};
use super::{Workspace, sha256_hex};
use crate::error::Result;
use crate::imports::{self, Import};
use crate::path::{Language, ProjectPath};

impl Workspace {
    /// The digest over every project module `path` imports, directly or
    /// through other modules (`Supermath.*` for Lean, `work.*` for Python).
    ///
    /// `lean_check` and `py_run` report it as `imports_sha256`: a verdict is
    /// about a file *and* what it imports, so it goes stale when either
    /// changes. A module with no file contributes `missing`, so creating it
    /// changes the digest. The file itself is not included; its own digest is
    /// [`Workspace::sha256`].
    ///
    /// # Errors
    ///
    /// As [`Workspace::read`] for `path`; imported files that are missing or
    /// unreadable count as missing rather than failing.
    pub fn imports_digest(&self, slug: &str, path: &ProjectPath) -> Result<ImportsDigest> {
        let language = path.language();
        let root = self.read(slug, path)?;
        let mut modules: BTreeMap<String, Option<String>> = BTreeMap::new();
        let mut queue: VecDeque<Import> = imports::parse(language, &root.text).into();
        while let Some(import) = queue.pop_front() {
            for candidate in candidates(language, &import) {
                if modules.contains_key(&candidate) || candidate == path.module() {
                    continue;
                }
                let loaded = ProjectPath::from_module(language, &candidate)
                    .and_then(|p| self.read(slug, &p).ok());
                let is_name = import.module != candidate;
                match loaded {
                    Some(file) => {
                        queue.extend(imports::parse(language, &file.text));
                        modules.insert(candidate, Some(file.sha256));
                    }
                    // `from work.a import name`: `name` is usually not a module.
                    None if is_name => {}
                    None if self.is_package_dir(slug, language, &candidate)? => {}
                    None => {
                        modules.insert(candidate, None);
                    }
                }
            }
        }
        let modules: Vec<ImportedModule> = modules
            .into_iter()
            .map(|(module, sha256)| ImportedModule { module, sha256 })
            .collect();
        let mut lines = String::new();
        for m in &modules {
            // Writing to a String cannot fail.
            let _ = writeln!(
                lines,
                "{}\t{}",
                m.module,
                m.sha256.as_deref().unwrap_or("missing")
            );
        }
        Ok(ImportsDigest {
            sha256: sha256_hex(lines.as_bytes()),
            modules,
        })
    }

    /// Whether `module` is a Python package directory (a namespace package
    /// with no file of its own).
    fn is_package_dir(&self, slug: &str, language: Language, module: &str) -> Result<bool> {
        if language != Language::Python {
            return Ok(false);
        }
        let relative = format!("{}/{}", language.root_dir(), module.replace('.', "/"));
        let dir = self.resolve_relative(slug, &relative)?;
        Ok(dir.is_dir())
    }
}

/// The project modules an import may refer to: the module itself, and for a
/// `from` import each imported name as a submodule. Foreign modules yield
/// none, and a bare `work` is a package, not a file.
fn candidates(language: Language, import: &Import) -> Vec<String> {
    if !imports::is_project_module(language, &import.module) {
        return Vec::new();
    }
    let mut found = Vec::new();
    if import.module != language.package_dir() {
        found.push(import.module.clone());
    }
    found.extend(
        import
            .names
            .iter()
            .map(|n| format!("{}.{n}", import.module)),
    );
    found
}
