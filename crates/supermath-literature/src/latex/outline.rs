//! The section tree of a flattened paper, with LaTeX's own numbering.

use super::scan::{brace_arg, bracket_arg, command_args, control_word, past_command};

/// One `\section`, `\subsection` or `\subsubsection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// 1 for a section, 2 for a subsection, 3 for a subsubsection.
    pub level: u8,
    /// The number LaTeX prints (`3`, `3.2`, `A.1`), or `None` for a starred
    /// heading, which is not numbered.
    pub number: Option<String>,
    /// The title as written, minus any `\label`.
    pub title: String,
    /// Byte offset of the heading command in the flattened text.
    pub start: usize,
    /// Byte offset just past the heading command, where its text begins.
    pub body_start: usize,
}

/// The headings of a paper, in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outline {
    /// Every heading, in document order.
    pub headings: Vec<Heading>,
    /// Byte offset of `\end{document}`, or the end of the text.
    pub end: usize,
}

fn level_of(name: &str) -> Option<u8> {
    match name {
        "section" => Some(1),
        "subsection" => Some(2),
        "subsubsection" => Some(3),
        _ => None,
    }
}

/// Numbers sections the way LaTeX does, switching to letters at `\appendix`.
fn format_number(counters: [u32; 3], level: u8, appendix: bool) -> String {
    let top = if appendix {
        char::from_u32(u32::from(b'A') + counters[0].saturating_sub(1))
            .map_or_else(|| counters[0].to_string(), String::from)
    } else {
        counters[0].to_string()
    };
    let mut parts = vec![top];
    parts.extend(counters[1..usize::from(level)].iter().map(u32::to_string));
    parts.join(".")
}

/// Scans `text` for headings.
///
/// ```
/// use supermath_literature::outline;
///
/// let o = outline("\\section{A}\\subsection{B}\\section*{C}\\section{D}");
/// let numbers: Vec<_> = o.headings.iter().map(|h| h.number.clone()).collect();
/// assert_eq!(numbers, [Some("1".into()), Some("1.1".into()), None, Some("2".into())]);
/// ```
#[must_use]
pub fn outline(text: &str) -> Outline {
    let end = text.find("\\end{document}").unwrap_or(text.len());
    let text = &text[..end];
    let mut headings = Vec::new();
    let mut counters = [0u32; 3];
    let mut appendix = false;
    let mut i = 0;
    while let Some(p) = text[i..].find('\\') {
        let at = i + p;
        let (name, word_end) = control_word(text, at);
        i = past_command(text, at, word_end);
        if name == "appendix" {
            appendix = true;
            counters = [0; 3];
            continue;
        }
        let Some(level) = level_of(name) else {
            continue;
        };
        let starred = text[word_end..].starts_with('*');
        let mut j = word_end + usize::from(starred);
        if let Some((_, after)) = bracket_arg(text, j) {
            j = after;
        }
        let Some((title, after)) = brace_arg(text, j) else {
            continue;
        };
        let number = if starred {
            None
        } else {
            let index = usize::from(level) - 1;
            counters[index] += 1;
            for c in &mut counters[index + 1..] {
                *c = 0;
            }
            Some(format_number(counters, level, appendix))
        };
        headings.push(Heading {
            level,
            number,
            title: strip_label(title),
            start: at,
            body_start: after,
        });
        i = after;
    }
    Outline { headings, end }
}

fn strip_label(title: &str) -> String {
    let mut out = title.to_owned();
    for label in command_args(title, &["label"]) {
        out = out.replace(&format!("\\label{{{label}}}"), "");
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Outline {
    /// The index of the heading numbered `number`.
    #[must_use]
    pub fn find(&self, number: &str) -> Option<usize> {
        self.headings
            .iter()
            .position(|h| h.number.as_deref() == Some(number))
    }

    /// The text under heading `index`, up to the next heading of the same or
    /// a higher level, trimmed. Includes its subsections.
    #[must_use]
    pub fn section_text<'a>(&self, text: &'a str, index: usize) -> &'a str {
        let Some(heading) = self.headings.get(index) else {
            return "";
        };
        let stop = self.headings[index + 1..]
            .iter()
            .find(|h| h.level <= heading.level)
            .map_or(self.end, |h| h.start);
        text[heading.body_start.min(stop)..stop].trim()
    }

    /// The heading in force at byte offset `pos`: the last one starting
    /// before it.
    #[must_use]
    pub fn current(&self, pos: usize) -> Option<&Heading> {
        self.headings.iter().rev().find(|h| h.start < pos)
    }
}
