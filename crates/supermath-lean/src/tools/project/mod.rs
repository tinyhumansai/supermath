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

/// A Lean file the workspace lists but cannot read, and why.
#[derive(Debug, Clone)]
pub(super) struct Unreadable {
    pub(super) path: ProjectPath,
    pub(super) reason: String,
}

/// The project's Lean files: the readable ones in dependency order, and the
/// ones that could not be read (too large, not UTF-8), which cannot be
/// checked either and are reported rather than dropped.
#[derive(Debug, Clone, Default)]
pub(super) struct LeanFiles {
    pub(super) ordered: Vec<ProjectFile>,
    pub(super) unreadable: Vec<Unreadable>,
}

/// Every Lean file of project `slug`, each readable one after the project
/// modules it imports. An import cycle is broken at the point it is found;
/// Lean itself will report it.
pub(super) fn lean_files(workspace: &Workspace, slug: &str) -> Result<LeanFiles> {
    let mut files: BTreeMap<String, ProjectFile> = BTreeMap::new();
    let mut unreadable = Vec::new();
    for path in workspace.list(slug)? {
        if path.language() != Language::Lean {
            continue;
        }
        match workspace.read(slug, &path) {
            Ok(file) => {
                files.insert(
                    path.module(),
                    ProjectFile {
                        path,
                        text: file.text,
                    },
                );
            }
            Err(why) => unreadable.push(Unreadable {
                path,
                reason: why.to_string(),
            }),
        }
    }
    let mut ordered = Vec::with_capacity(files.len());
    let mut seen = BTreeSet::new();
    for module in files.keys() {
        visit(module, &files, &mut seen, &mut ordered);
    }
    Ok(LeanFiles {
        ordered,
        unreadable,
    })
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
