//! The outline of a Lean or Python file: its declarations without their text.
//!
//! Most turns need the outline, not the file. For Lean it lists each
//! declaration with its kind, line range, and whether its proof contains a
//! `sorry`; for Python, each top-level function and class with its signature
//! and line range. The scan is line based, not a full parse: a declaration
//! starts at a line in column zero and runs to the line before the next
//! top-level command.
//!
//! ```
//! use supermath_files::{Language, outline};
//!
//! let items = outline::outline(Language::Lean, "theorem t : 1 = 1 := by\n  sorry\n");
//! assert_eq!(items[0].name, "t");
//! assert_eq!(items[0].has_sorry, Some(true));
//! ```

use serde::Serialize;

use crate::imports::strip_lean_comments;
use crate::path::Language;

/// One declaration in a file's outline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutlineItem {
    /// The declaration's kind: `theorem`, `def`, `structure`, … for Lean;
    /// `function`, `async_function`, or `class` for Python.
    pub kind: String,
    /// The declared name; `<anonymous>` for an unnamed `instance` or
    /// `example`.
    pub name: String,
    /// One-based first line.
    pub start_line: usize,
    /// One-based last line.
    pub end_line: usize,
    /// Python only: the `def`/`class` line, joined over continuation lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Lean only: whether the declaration's text contains `sorry`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_sorry: Option<bool>,
}

/// The outline of `source`, a file of `language`.
#[must_use]
pub fn outline(language: Language, source: &str) -> Vec<OutlineItem> {
    match language {
        Language::Lean => lean_outline(source),
        Language::Python => python_outline(source),
    }
}

const LEAN_DECLARATIONS: &[&str] = &[
    "theorem",
    "lemma",
    "def",
    "abbrev",
    "instance",
    "structure",
    "inductive",
    "class",
    "axiom",
    "example",
    "opaque",
];
const LEAN_MODIFIERS: &[&str] = &[
    "private",
    "protected",
    "noncomputable",
    "partial",
    "unsafe",
    "nonrec",
];
const LEAN_COMMANDS: &[&str] = &[
    "end",
    "namespace",
    "section",
    "open",
    "variable",
    "universe",
    "import",
    "set_option",
    "attribute",
    "notation",
    "macro",
    "syntax",
    "local",
    "scoped",
    "deriving",
    "export",
    "mutual",
    "#check",
    "#eval",
    "#print",
    "#reduce",
    "infix",
    "infixl",
    "infixr",
    "prefix",
    "postfix",
    "elab",
    "declare_syntax_cat",
    "initialize",
];

/// The declaration at a column-zero Lean line: its kind and name.
fn lean_declaration(line: &str) -> Option<(String, String)> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let mut words = line.split_whitespace();
    let mut word = words.next()?;
    while LEAN_MODIFIERS.contains(&word) || (word.starts_with("@[") && word.ends_with(']')) {
        word = words.next()?;
    }
    let kind = LEAN_DECLARATIONS.iter().find(|k| **k == word)?;
    let name = words
        .next()
        .filter(|n| !n.starts_with([':', '(', '[', '{', '⦃']))
        .unwrap_or("<anonymous>");
    Some(((*kind).to_owned(), name.to_owned()))
}

fn starts_lean_command(line: &str) -> bool {
    if line.is_empty() || line.starts_with(char::is_whitespace) {
        return false;
    }
    let first = line.split_whitespace().next().unwrap_or_default();
    lean_declaration(line).is_some()
        || LEAN_COMMANDS
            .iter()
            .any(|c| first == *c || first.starts_with("@["))
}

fn lean_outline(source: &str) -> Vec<OutlineItem> {
    let cleaned = strip_lean_comments(source);
    let lines: Vec<&str> = cleaned.lines().collect();
    let mut items = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((kind, name)) = lean_declaration(line) else {
            continue;
        };
        let mut end = index + 1;
        while end < lines.len() && !starts_lean_command(lines[end]) {
            end += 1;
        }
        while end > index + 1 && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        let has_sorry = lines[index..end].iter().any(|l| {
            l.split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|w| w == "sorry")
        });
        items.push(OutlineItem {
            kind,
            name,
            start_line: index + 1,
            end_line: end,
            signature: None,
            has_sorry: Some(has_sorry),
        });
    }
    items
}

fn python_outline(source: &str) -> Vec<OutlineItem> {
    let lines: Vec<&str> = source.lines().collect();
    let mut items = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let (kind, rest) = if let Some(rest) = line.strip_prefix("async def ") {
            ("async_function", rest)
        } else if let Some(rest) = line.strip_prefix("def ") {
            ("function", rest)
        } else if let Some(rest) = line.strip_prefix("class ") {
            ("class", rest)
        } else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let mut signature = line.trim_end().to_owned();
        let mut last = index;
        while !signature
            .split('#')
            .next()
            .unwrap_or_default()
            .trim_end()
            .ends_with(':')
            && last + 1 < lines.len()
        {
            last += 1;
            signature.push(' ');
            signature.push_str(lines[last].trim());
        }
        let mut end = last + 1;
        while end < lines.len() && !ends_python_block(lines[end]) {
            end += 1;
        }
        while end > last + 1 && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        items.push(OutlineItem {
            kind: kind.to_owned(),
            name,
            start_line: index + 1,
            end_line: end,
            signature: Some(signature),
            has_sorry: None,
        });
    }
    items
}

/// Whether a line at this position starts a new top-level statement.
fn ends_python_block(line: &str) -> bool {
    line.chars()
        .next()
        .is_some_and(|c| !c.is_whitespace() && !matches!(c, ')' | ']' | '}'))
}

#[cfg(test)]
mod test;
