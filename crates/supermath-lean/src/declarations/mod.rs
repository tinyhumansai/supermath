//! The declarations a Lean file makes, with their fully qualified names.
//!
//! `supermath-files` outlines a file by the names as written; Lean checks and
//! graphs need the names as the environment knows them, so this module adds
//! the `namespace` and `section` scopes in force at each declaration. The scan
//! is line based like the outline: it reads column-zero commands, skips
//! comments, and does not elaborate. `lean_check` uses it to choose the
//! declarations whose axioms are reported, and `lean_blueprint` to name its
//! nodes.
//!
//! ```
//! use supermath_lean::declarations;
//!
//! let source = "namespace Sendov\ntheorem main : True := trivial\nend Sendov\n";
//! let found = declarations(source);
//! assert_eq!(found[0].name, "Sendov.main");
//! assert_eq!(found[0].line, 2);
//! ```

use std::collections::BTreeMap;

use supermath_files::{Language, outline};

/// One declaration in a Lean file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The fully qualified name, such as `Sendov.main`.
    pub name: String,
    /// The declaring keyword: `theorem`, `def`, `structure`, ….
    pub kind: String,
    /// One-based first line.
    pub line: usize,
    /// Whether the declaration's own text contains `sorry`.
    pub has_sorry: bool,
}

/// The named declarations of `source`, in file order.
///
/// `example`s and unnamed `instance`s have no name to report axioms for and
/// are left out.
#[must_use]
pub fn declarations(source: &str) -> Vec<Declaration> {
    let items: BTreeMap<usize, _> = outline::outline(Language::Lean, source)
        .into_iter()
        .map(|item| (item.start_line, item))
        .collect();
    let cleaned = strip_comments(source);
    let mut scopes: Vec<Vec<String>> = Vec::new();
    let mut found = Vec::new();
    for (index, line) in cleaned.lines().enumerate() {
        let number = index + 1;
        if let Some(item) = items.get(&number) {
            if item.kind != "example" && item.name != "<anonymous>" {
                found.push(Declaration {
                    name: qualify(&scopes, &item.name),
                    kind: item.kind.clone(),
                    line: number,
                    has_sorry: item.has_sorry.unwrap_or(false),
                });
            }
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        let mut words = line.split_whitespace();
        match words.next() {
            Some("namespace") => scopes.push(
                words
                    .next()
                    .map(|n| n.split('.').map(str::to_owned).collect())
                    .unwrap_or_default(),
            ),
            Some("section" | "mutual") => scopes.push(Vec::new()),
            Some("end") => {
                scopes.pop();
            }
            _ => {}
        }
    }
    found
}

fn qualify(scopes: &[Vec<String>], name: &str) -> String {
    if let Some(root) = name.strip_prefix("_root_.") {
        return root.to_owned();
    }
    let mut parts: Vec<&str> = scopes.iter().flatten().map(String::as_str).collect();
    parts.push(name);
    parts.join(".")
}

/// Replaces `--` line comments and nested `/- -/` block comments with blanks,
/// keeping every newline so line numbers survive.
fn strip_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let (mut i, mut depth) = (0, 0usize);
    while i < bytes.len() {
        let rest = &bytes[i..];
        if rest.starts_with(b"/-") {
            depth += 1;
            i += 2;
        } else if depth > 0 && rest.starts_with(b"-/") {
            depth -= 1;
            i += 2;
        } else if depth == 0 && rest.starts_with(b"--") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else {
            if depth == 0 || bytes[i] == b'\n' {
                out.push(bytes[i]);
            }
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod test;
