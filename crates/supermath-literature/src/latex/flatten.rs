//! Flattening: `\input` inlined, comments gone, simple macros expanded.

use std::collections::HashMap;

use super::scan::{brace_arg, bracket_arg, control_word, past_command, skip_ws, strip_comments};
use super::tree::SourceTree;

/// The deepest `\input` nesting followed.
const MAX_DEPTH: usize = 16;
/// How many rounds of macro expansion run (macros may use macros).
const MAX_PASSES: usize = 4;

/// A paper's source as one string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flattened {
    /// The main file with every resolvable `\input` inlined, comments removed,
    /// and argument-free macro definitions removed and expanded.
    pub text: String,
    /// The path of the main file inside the tree.
    pub main: String,
    /// What could not be resolved: a missing `\input` file, an `\input` cycle.
    pub unknowns: Vec<String>,
    /// How many argument-free macros were expanded.
    pub macros: usize,
}

/// Flattens an e-print's LaTeX files into one string.
///
/// ```
/// use supermath_literature::{SourceTree, flatten_latex};
///
/// let mut tree = SourceTree::new();
/// tree.insert("main.tex", "\\newcommand{\\R}{\\mathbb{R}}\n$x\\in\\R$ % note\n\\input{a}".into());
/// tree.insert("a.tex", "more".into());
/// let flat = flatten_latex(&tree);
/// assert_eq!(flat.text.trim(), "$x\\in\\mathbb{R}$ \n\nmore");
/// ```
#[must_use]
pub fn flatten_latex(tree: &SourceTree) -> Flattened {
    let mut unknowns = Vec::new();
    let Some(main) = tree.main_file() else {
        unknowns.push("the source has no .tex files".to_owned());
        return Flattened {
            text: String::new(),
            main: String::new(),
            unknowns,
            macros: 0,
        };
    };
    let mut stack = vec![main.to_owned()];
    let inlined = inline(tree, main, &mut stack, &mut unknowns);
    let (stripped, macros) = take_macros(&inlined);
    let count = macros.len();
    let mut text = stripped;
    for _ in 0..MAX_PASSES {
        let next = expand(&text, &macros);
        if next == text {
            break;
        }
        text = next;
    }
    Flattened {
        text,
        main: main.to_owned(),
        unknowns,
        macros: count,
    }
}

fn inline(
    tree: &SourceTree,
    path: &str,
    stack: &mut Vec<String>,
    unknowns: &mut Vec<String>,
) -> String {
    let text = strip_comments(tree.get(path).unwrap_or_default());
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(p) = text[i..].find('\\') {
        let at = i + p;
        let (name, end) = control_word(&text, at);
        let target = if matches!(name, "input" | "include") {
            brace_arg(&text, end)
        } else {
            None
        };
        let Some((arg, after)) = target else {
            let stop = past_command(&text, at, end);
            out.push_str(&text[i..stop]);
            i = stop;
            continue;
        };
        out.push_str(&text[i..at]);
        i = after;
        let name = arg.trim();
        match tree.resolve(name, path) {
            None => unknowns.push(format!("\\input file {name} is not in the source")),
            Some(file) if stack.contains(&file) => {
                unknowns.push(format!("\\input cycle through {file}"));
            }
            Some(_) if stack.len() >= MAX_DEPTH => {
                unknowns.push(format!(
                    "\\input nesting deeper than {MAX_DEPTH} stopped at {name}"
                ));
            }
            Some(file) => {
                stack.push(file.clone());
                out.push('\n');
                out.push_str(&inline(tree, &file, stack, unknowns));
                out.push('\n');
                stack.pop();
            }
        }
    }
    out.push_str(&text[i..]);
    out
}

/// Removes argument-free macro definitions, returning them by name.
fn take_macros(text: &str) -> (String, HashMap<String, String>) {
    let mut out = String::with_capacity(text.len());
    let mut macros = HashMap::new();
    let mut i = 0;
    while let Some(p) = text[i..].find('\\') {
        let at = i + p;
        let (name, end) = control_word(text, at);
        let parsed = match name {
            "newcommand" | "providecommand" => newcommand(text, end),
            "def" => def(text, end),
            "DeclareMathOperator" => declare_operator(text, end),
            _ => None,
        };
        if let Some((macro_name, body, after)) = parsed {
            out.push_str(&text[i..at]);
            macros.insert(macro_name, body);
            i = after;
        } else {
            let stop = past_command(text, at, end);
            out.push_str(&text[i..stop]);
            i = stop;
        }
    }
    out.push_str(&text[i..]);
    (out, macros)
}

/// A macro name worth expanding. Names starting `the` are counters such as
/// `\thetheorem`, which the statement parser reads as written.
fn usable(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphabetic()) && !name.starts_with("the")
}

fn macro_name(text: &str, i: usize) -> Option<(&str, usize)> {
    let i = skip_ws(text, i);
    if let Some((inner, after)) = brace_arg(text, i) {
        let inner = inner.trim();
        let name = inner.strip_prefix('\\')?;
        return Some((name, after));
    }
    if text[i..].starts_with('\\') {
        let (name, end) = control_word(text, i);
        return Some((name, end));
    }
    None
}

fn newcommand(text: &str, end: usize) -> Option<(String, String, usize)> {
    let end = if text[end..].starts_with('*') {
        end + 1
    } else {
        end
    };
    let (name, after) = macro_name(text, end)?;
    if !usable(name) || bracket_arg(text, after).is_some() {
        return None;
    }
    let (body, after) = brace_arg(text, after)?;
    Some((name.to_owned(), body.to_owned(), after))
}

fn def(text: &str, end: usize) -> Option<(String, String, usize)> {
    let i = skip_ws(text, end);
    if !text[i..].starts_with('\\') {
        return None;
    }
    let (name, after) = control_word(text, i);
    if !usable(name) || !text[after..].starts_with('{') {
        return None;
    }
    let (body, after) = brace_arg(text, after)?;
    Some((name.to_owned(), body.to_owned(), after))
}

fn declare_operator(text: &str, end: usize) -> Option<(String, String, usize)> {
    let end = if text[end..].starts_with('*') {
        end + 1
    } else {
        end
    };
    let (name, after) = macro_name(text, end)?;
    if !usable(name) {
        return None;
    }
    let (word, after) = brace_arg(text, after)?;
    Some((name.to_owned(), format!("\\operatorname{{{word}}}"), after))
}

/// One round of expansion.
fn expand(text: &str, macros: &HashMap<String, String>) -> String {
    if macros.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(p) = text[i..].find('\\') {
        let at = i + p;
        let (name, end) = control_word(text, at);
        out.push_str(&text[i..at]);
        if let Some(body) = macros.get(name) {
            out.push_str(body);
            i = end;
        } else {
            let stop = past_command(text, at, end);
            out.push_str(&text[at..stop]);
            i = stop;
        }
    }
    out.push_str(&text[i..]);
    out
}
