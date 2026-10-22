//! Deterministic extraction of numbered statements from flattened LaTeX.
//!
//! The parser reads `\newtheorem` declarations (plain, shared-counter
//! `[theorem]`, within-section `[section]`, and starred), then walks the text
//! once, counting the way LaTeX does, and emits one [`Statement`] per
//! declared environment. No model is involved. What it cannot number, it does
//! not guess: it lists the environment in [`Statements::unknowns`].

mod types;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::latex::scan::{
    brace_arg, bracket_arg, command_args, control_word, find_end, past_command, skip_ws,
};
use crate::latex::{Outline, outline};

use types::Declaration;
pub use types::{Statement, Statements};

/// Environments every theorem-like document uses. One of these in the text
/// with no `\newtheorem` for it is reported, not numbered: the class file may
/// define it, and its numbering is then not in the paper.
const STANDARD: [&str; 7] = [
    "theorem",
    "lemma",
    "proposition",
    "corollary",
    "conjecture",
    "definition",
    "remark",
];

/// Commands whose braces name labels.
const REF_COMMANDS: [&str; 6] = ["ref", "eqref", "cref", "Cref", "autoref", "vref"];

/// Extracts the numbered statements of a flattened paper.
///
/// ```
/// use supermath_literature::extract_statements;
///
/// let text = "\\newtheorem{thm}{Theorem}\\newtheorem{lem}[thm]{Lemma}\n\
///     \\section{A}\\begin{lem}\\label{l}Small.\\end{lem}\n\
///     \\begin{thm}\\label{t}Uses \\ref{l}.\\end{thm}";
/// let out = extract_statements(text);
/// let numbers: Vec<_> = out.statements.iter().map(|s| s.number.clone()).collect();
/// assert_eq!(numbers, [Some("1".into()), Some("2".into())]);
/// assert_eq!(out.statements[1].refs, ["l"]);
/// ```
#[must_use]
pub fn extract_statements(text: &str) -> Statements {
    let declarations = declarations(text);
    let outline = outline(text);
    let mut walker = Walker {
        text,
        declarations: &declarations,
        outline: &outline,
        counters: HashMap::new(),
        custom: BTreeSet::new(),
        undeclared: BTreeMap::new(),
        out: Statements::default(),
    };
    walker.walk();
    walker.finish()
}

fn declarations(text: &str) -> HashMap<String, Declaration> {
    let mut found = HashMap::new();
    let mut i = 0;
    while let Some(p) = text[i..].find('\\') {
        let at = i + p;
        let (name, end) = control_word(text, at);
        i = past_command(text, at, end);
        if name != "newtheorem" {
            continue;
        }
        let starred = text[end..].starts_with('*');
        let Some((env, after)) = brace_arg(text, end + usize::from(starred)) else {
            continue;
        };
        let (shared, after) = optional(text, after);
        let Some((title, after)) = brace_arg(text, after) else {
            continue;
        };
        let (within, after) = optional(text, after);
        let env = env.trim().to_owned();
        found.insert(
            env.clone(),
            Declaration {
                env,
                title: title.split_whitespace().collect::<Vec<_>>().join(" "),
                starred,
                shared,
                within,
            },
        );
        i = after;
    }
    found
}

/// A `[word]` at `i` (after whitespace), and where reading resumes.
fn optional(text: &str, i: usize) -> (Option<String>, usize) {
    match bracket_arg(text, i) {
        Some((inner, after)) => (Some(inner.trim().to_owned()), after),
        None => (None, i),
    }
}

struct Walker<'a> {
    text: &'a str,
    declarations: &'a HashMap<String, Declaration>,
    outline: &'a Outline,
    /// Counter name to (value, reset key).
    counters: HashMap<String, (u32, Option<usize>)>,
    /// Counters whose `\the<counter>` the paper redefines.
    custom: BTreeSet<String>,
    undeclared: BTreeMap<String, usize>,
    out: Statements,
}

impl Walker<'_> {
    fn walk(&mut self) {
        let text = self.text;
        let mut i = 0;
        while let Some(p) = text[i..].find('\\') {
            let at = i + p;
            let (name, end) = control_word(text, at);
            i = past_command(text, at, end);
            if name != "begin" {
                continue;
            }
            let Some((env, body_start)) = brace_arg(text, end) else {
                continue;
            };
            let env = env.trim();
            if let Some(declaration) = self.declarations.get(env) {
                match find_end(text, body_start, env) {
                    Some(stop) => {
                        self.statement(declaration, at, body_start, stop);
                        i = stop;
                    }
                    None => self
                        .out
                        .unknowns
                        .push(format!("\\begin{{{env}}} at byte {at} has no \\end")),
                }
            } else if STANDARD.contains(&env) {
                *self.undeclared.entry(env.to_owned()).or_default() += 1;
            }
        }
    }

    fn finish(mut self) -> Statements {
        for (env, count) in &self.undeclared {
            self.out.unknowns.push(format!(
                "{count} \\begin{{{env}}} with no \\newtheorem in the source; its numbering is defined elsewhere"
            ));
        }
        for counter in &self.custom {
            self.out.unknowns.push(format!(
                "counter {counter} is renumbered with \\the{counter}; its statements are not numbered"
            ));
        }
        self.out
    }

    /// The counter an environment counts with, following `[shared]` links.
    fn counter_owner<'d>(&'d self, declaration: &'d Declaration) -> Option<&'d Declaration> {
        let mut current = declaration;
        for _ in 0..8 {
            match &current.shared {
                None => return Some(current),
                Some(next) => current = self.declarations.get(next)?,
            }
        }
        None
    }

    fn redefines(&self, counter: &str) -> bool {
        [
            format!("\\renewcommand{{\\the{counter}}}"),
            format!("\\renewcommand\\the{counter}"),
            format!("\\def\\the{counter}"),
        ]
        .iter()
        .any(|pattern| self.text.contains(pattern.as_str()))
    }

    /// The reset key and number prefix for a `within` level, from the
    /// headings in force at `pos`.
    fn scope(&self, within: &str, pos: usize) -> Option<(Option<usize>, String)> {
        let max_level = match within {
            "section" => 1,
            "subsection" => 2,
            _ => return None,
        };
        let numbered = self
            .outline
            .headings
            .iter()
            .enumerate()
            .rev()
            .find(|(_, h)| h.start < pos && h.level <= max_level && h.number.is_some());
        Some(match numbered {
            None => (None, "0".to_owned()),
            Some((index, h)) => {
                let number = h.number.clone().unwrap_or_default();
                let prefix = if h.level < max_level {
                    format!("{number}.0")
                } else {
                    number
                };
                (Some(index), prefix)
            }
        })
    }

    /// `Err(None)` skips the statement silently (it is reported once, later);
    /// `Err(Some(why))` skips it with a reason.
    fn number(
        &mut self,
        declaration: &Declaration,
        pos: usize,
    ) -> Result<Option<String>, Option<String>> {
        if declaration.starred {
            return Ok(None);
        }
        let owner = self
            .counter_owner(declaration)
            .ok_or_else(|| {
                Some(format!(
                    "{} shares a counter that is not a theorem environment",
                    declaration.env
                ))
            })?
            .clone();
        if self.redefines(&owner.env) {
            self.custom.insert(owner.env.clone());
            return Err(None);
        }
        let scope = match &owner.within {
            Some(within) => Some(self.scope(within, pos).ok_or_else(|| {
                Some(format!(
                    "{} is numbered within {within}, which is not supported",
                    owner.env
                ))
            })?),
            None => None,
        };
        let entry = self.counters.entry(owner.env.clone()).or_insert((0, None));
        let key = scope.as_ref().and_then(|(key, _)| *key);
        if entry.1 != key {
            *entry = (0, key);
        }
        entry.0 += 1;
        Ok(Some(match scope {
            Some((_, prefix)) => format!("{prefix}.{}", entry.0),
            None => entry.0.to_string(),
        }))
    }

    fn statement(&mut self, declaration: &Declaration, at: usize, body_start: usize, stop: usize) {
        let number = match self.number(declaration, at) {
            Ok(number) => number,
            Err(reason) => {
                self.out.unknowns.extend(reason);
                return;
            }
        };
        let text = self.text;
        // `\begin{thm}[Name]`: the optional name is not part of the statement.
        let inner_start = bracket_arg(text, body_start)
            .filter(|(_, after)| *after <= stop)
            .map_or(body_start, |(_, after)| after);
        let body = &text[inner_start..stop];
        let after_end = stop + format!("\\end{{{}}}", declaration.env).len();
        let proof = proof_after(text, after_end, body);
        let label = command_args(body, &["label"])
            .first()
            .map(|l| (*l).trim().to_owned());
        let mut refs: Vec<String> = Vec::new();
        for scope in [Some(body), proof].into_iter().flatten() {
            for arg in command_args(scope, &REF_COMMANDS) {
                for one in arg.split(',').map(str::trim).filter(|l| !l.is_empty()) {
                    if Some(one) != label.as_deref() && !refs.iter().any(|r| r == one) {
                        refs.push(one.to_owned());
                    }
                }
            }
        }
        let mut statement = body.to_owned();
        if let Some(label) = &label {
            statement = statement.replacen(&format!("\\label{{{label}}}"), "", 1);
        }
        self.out.statements.push(Statement {
            label,
            kind: declaration.title.to_lowercase(),
            env: declaration.env.clone(),
            number,
            section: self.outline.current(at).and_then(|h| h.number.clone()),
            statement: statement.trim().to_owned(),
            refs,
        });
    }
}

/// The proof that directly follows a statement, if it is its proof: a `proof`
/// whose title names another label belongs to that statement.
fn proof_after<'t>(text: &'t str, from: usize, statement: &str) -> Option<&'t str> {
    let start = skip_ws(text, from);
    let open = "\\begin{proof}";
    if !text[start..].starts_with(open) {
        return None;
    }
    let body_start = start + open.len();
    let stop = find_end(text, body_start, "proof")?;
    if let Some((title, _)) = bracket_arg(text, body_start).filter(|(_, after)| *after <= stop) {
        let named = command_args(title, &REF_COMMANDS);
        let own = command_args(statement, &["label"]);
        if !named.is_empty() && !named.iter().any(|n| own.contains(n)) {
            return None;
        }
    }
    Some(&text[body_start..stop])
}

#[cfg(test)]
mod test;
