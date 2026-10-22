//! The project's Lean files in dependency order.

use std::collections::{BTreeMap, BTreeSet};

use supermath_files::{Language, ProjectPath, Workspace, imports};

use crate::error::Result;

/// A Lean file of the project and its text.
#[derive(Debug, Clone)]
pub(super) struct ProjectFile {
    pub(super) path: ProjectPath,
    pub(super) text: String,
}

/// Every readable Lean file of project `slug`, each after the project modules
/// it imports. Files that cannot be read (too large, not UTF-8) are skipped:
/// they cannot be checked either. An import cycle is broken at the point it is
/// found; Lean itself will report it.
pub(super) fn lean_files(workspace: &Workspace, slug: &str) -> Result<Vec<ProjectFile>> {
    let mut files: BTreeMap<String, ProjectFile> = BTreeMap::new();
    for path in workspace.list(slug)? {
        if path.language() != Language::Lean {
            continue;
        }
        if let Ok(file) = workspace.read(slug, &path) {
            files.insert(
                path.module(),
                ProjectFile {
                    path,
                    text: file.text,
                },
            );
        }
    }
    let mut ordered = Vec::with_capacity(files.len());
    let mut seen = BTreeSet::new();
    for module in files.keys() {
        visit(module, &files, &mut seen, &mut ordered);
    }
    Ok(ordered)
}

fn visit(
    module: &str,
    files: &BTreeMap<String, ProjectFile>,
    seen: &mut BTreeSet<String>,
    ordered: &mut Vec<ProjectFile>,
) {
    if !seen.insert(module.to_owned()) {
        return;
    }
    let Some(file) = files.get(module) else {
        return;
    };
    for import in imports::parse(Language::Lean, &file.text) {
        visit(&import.module, files, seen, ordered);
    }
    ordered.push(file.clone());
}

#[cfg(test)]
mod test;
