//! The files of an e-print.

use std::collections::BTreeMap;

/// The `.tex` files of one e-print, by path.
///
/// ```
/// use supermath_literature::SourceTree;
///
/// let mut tree = SourceTree::new();
/// tree.insert("./paper/main.tex", "\\input{intro}".to_owned());
/// assert_eq!(tree.paths(), vec!["paper/main.tex"]);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceTree {
    files: BTreeMap<String, String>,
}

/// Removes `.` and empty segments and resolves `..`.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

impl SourceTree {
    /// An empty tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file; the path is normalised (`./a/../b.tex` becomes `b.tex`).
    pub fn insert(&mut self, path: &str, text: String) {
        self.files.insert(normalize(path), text);
    }

    /// The text at `path`, normalised.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(&normalize(path)).map(String::as_str)
    }

    /// Every path, sorted.
    #[must_use]
    pub fn paths(&self) -> Vec<&str> {
        self.files.keys().map(String::as_str).collect()
    }

    /// Whether the tree holds no files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Resolves an `\input{name}` written in the file `from`: relative to
    /// that file's directory first, then to the root, with and without `.tex`.
    pub(crate) fn resolve(&self, name: &str, from: &str) -> Option<String> {
        let dir = from.rsplit_once('/').map_or("", |(d, _)| d);
        for base in [dir, ""] {
            let joined = if base.is_empty() {
                name.to_owned()
            } else {
                format!("{base}/{name}")
            };
            for candidate in [format!("{joined}.tex"), joined] {
                let candidate = normalize(&candidate);
                if self.files.contains_key(&candidate) {
                    return Some(candidate);
                }
            }
        }
        None
    }

    /// The path of the main file: the one with `\documentclass` and
    /// `\begin{document}`, else one with either, else the largest.
    pub(crate) fn main_file(&self) -> Option<&str> {
        let score = |text: &str| {
            u8::from(text.contains("\\documentclass"))
                + u8::from(text.contains("\\begin{document}"))
        };
        self.files
            .iter()
            .max_by_key(|(path, text)| {
                (score(text), text.len(), std::cmp::Reverse((*path).clone()))
            })
            .map(|(path, _)| path.as_str())
    }
}
