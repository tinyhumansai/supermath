//! [`extract_main_text`]: deterministic readable text from HTML.
//!
//! No model reads the page. Scripts, styles, navigation, hidden elements, and
//! comments are removed; headings, paragraphs, lists, code, and quotes are
//! kept as Markdown. Math is returned as TeX where the page carries it
//! (`MathJax` and `KaTeX` sources, `<annotation encoding="application/x-tex">`).
//! On Stack Exchange pages, answers come back with their score and
//! acceptance, highest first.

use scraper::{ElementRef, Html, Node};
use serde::Serialize;

/// Elements dropped with everything inside them.
const SKIPPED: [&str; 17] = [
    "script", "style", "noscript", "nav", "footer", "aside", "form", "svg", "template", "iframe",
    "head", "button", "select", "textarea", "input", "canvas", "object",
];

/// Elements that start and end a paragraph.
const BLOCKS: [&str; 20] = [
    "p",
    "div",
    "section",
    "article",
    "main",
    "ul",
    "ol",
    "table",
    "tr",
    "figure",
    "figcaption",
    "dl",
    "dd",
    "dt",
    "header",
    "details",
    "summary",
    "address",
    "fieldset",
    "body",
];

/// Whether and how a page's math survived extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MathKind {
    /// Math is returned as TeX.
    Tex,
    /// The page shows math but carries no TeX; the rendered glyphs are lost.
    Rendered,
    /// The page has no math.
    None,
}

/// The readable content of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    /// The page title.
    pub title: String,
    /// The main text as Markdown.
    pub text: String,
    /// What happened to the page's math.
    pub math: MathKind,
}

#[derive(Default)]
struct Walker {
    out: String,
    space: bool,
    tex: bool,
    rendered: bool,
}

fn attr<'a>(el: &ElementRef<'a>, name: &str) -> Option<&'a str> {
    el.value().attr(name)
}

fn has_class(el: &ElementRef<'_>, class: &str) -> bool {
    el.value().classes().any(|c| c == class)
}

fn is_hidden(el: &ElementRef<'_>) -> bool {
    let style = attr(el, "style")
        .unwrap_or("")
        .replace(' ', "")
        .to_ascii_lowercase();
    el.value().attr("hidden").is_some()
        || style.contains("display:none")
        || style.contains("visibility:hidden")
}

fn descendants(el: ElementRef<'_>) -> impl Iterator<Item = ElementRef<'_>> {
    el.descendants().filter_map(ElementRef::wrap)
}

/// The TeX source inside an element, if it carries one.
fn tex_source(el: ElementRef<'_>) -> Option<String> {
    descendants(el)
        .find(|d| {
            d.value().name() == "annotation"
                && attr(d, "encoding").is_some_and(|e| e.eq_ignore_ascii_case("application/x-tex"))
        })
        .map(|d| d.text().collect::<String>().trim().to_owned())
        .filter(|t| !t.is_empty())
}

fn is_rendered_math(el: &ElementRef<'_>) -> bool {
    el.value().name() == "math"
        || el.value().name() == "mjx-container"
        || has_class(el, "katex")
        || has_class(el, "MathJax")
        || has_class(el, "MathJax_Display")
}

impl Walker {
    fn block_break(&mut self) {
        self.trim_end_spaces();
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            while !self.out.ends_with("\n\n") {
                self.out.push('\n');
            }
        }
        self.space = false;
    }

    fn line_break(&mut self) {
        self.trim_end_spaces();
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.space = false;
    }

    fn trim_end_spaces(&mut self) {
        while self.out.ends_with(' ') {
            self.out.pop();
        }
    }

    fn push_inline(&mut self, text: &str) {
        if self.space && !self.out.is_empty() && !self.out.ends_with([' ', '\n']) {
            self.out.push(' ');
        }
        self.space = false;
        self.out.push_str(text);
    }

    fn text(&mut self, text: &str) {
        if text.starts_with(char::is_whitespace) {
            self.space = true;
        }
        for (i, word) in text.split_whitespace().enumerate() {
            if i > 0 {
                self.space = true;
            }
            self.push_inline(word);
        }
        if text.ends_with(char::is_whitespace) {
            self.space = true;
        }
    }

    fn children(&mut self, el: ElementRef<'_>) {
        for child in el.children() {
            match child.value() {
                Node::Text(text) => self.text(text),
                Node::Element(_) => {
                    if let Some(element) = ElementRef::wrap(child) {
                        self.element(element);
                    }
                }
                _ => {}
            }
        }
    }

    fn math(&mut self, el: ElementRef<'_>, display: bool) {
        if let Some(tex) = tex_source(el) {
            self.tex = true;
            let fence = if display { "$$" } else { "$" };
            self.push_inline(&format!("{fence}{tex}{fence}"));
        } else {
            self.rendered = true;
            let text: String = el.text().collect();
            self.text(&text);
        }
    }

    fn element(&mut self, el: ElementRef<'_>) {
        let name = el.value().name();
        if name == "script" {
            let kind = attr(&el, "type").unwrap_or("");
            if kind.starts_with("math/tex") {
                let tex: String = el.text().collect();
                self.tex = true;
                let fence = if kind.contains("mode=display") {
                    "$$"
                } else {
                    "$"
                };
                self.push_inline(&format!("{fence}{}{fence}", tex.trim()));
            }
            return;
        }
        if SKIPPED.contains(&name) || is_hidden(&el) || has_class(&el, "MathJax_Preview") {
            return;
        }
        if is_rendered_math(&el) {
            let display =
                has_class(&el, "MathJax_Display") || attr(&el, "display") == Some("block");
            self.math(el, display);
            return;
        }
        if attr(&el, "aria-hidden") == Some("true") {
            return;
        }
        match name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = usize::from(name.as_bytes()[1] - b'0');
                self.block_break();
                self.push_inline(&"#".repeat(level));
                self.out.push(' ');
                self.children(el);
                self.block_break();
            }
            "li" => {
                self.line_break();
                self.out.push_str("- ");
                self.space = false;
                self.children(el);
                self.line_break();
            }
            "pre" => {
                self.block_break();
                let code: String = el.text().collect();
                self.out.push_str("```\n");
                self.out.push_str(code.trim_matches('\n'));
                self.out.push_str("\n```");
                self.block_break();
            }
            "code" => {
                let code: String = el.text().collect();
                self.push_inline(&format!("`{}`", code.trim()));
            }
            "br" => self.line_break(),
            "blockquote" => {
                self.block_break();
                let at = self.out.len();
                self.children(el);
                let body = self.out.split_off(at);
                for line in body.trim().lines() {
                    self.out.push_str("> ");
                    self.out.push_str(line);
                    self.out.push('\n');
                }
                self.block_break();
            }
            "td" | "th" => {
                self.children(el);
                self.trim_end_spaces();
                self.out.push_str(" |");
                self.space = true;
            }
            _ if BLOCKS.contains(&name) => {
                self.block_break();
                self.children(el);
                self.block_break();
            }
            _ => self.children(el),
        }
    }

    fn finish(self) -> String {
        let mut lines: Vec<&str> = Vec::new();
        for line in self.out.lines().map(str::trim_end) {
            if line.is_empty() && lines.last().is_none_or(|l| l.is_empty()) {
                continue;
            }
            lines.push(line);
        }
        lines.join("\n").trim().to_owned()
    }
}

fn find_first<'a>(
    root: ElementRef<'a>,
    pred: impl Fn(&ElementRef<'a>) -> bool,
) -> Option<ElementRef<'a>> {
    descendants(root).find(|e| pred(e))
}

fn post_body(post: ElementRef<'_>) -> ElementRef<'_> {
    find_first(post, |e| {
        has_class(e, "js-post-body") || has_class(e, "post-text")
    })
    .unwrap_or(post)
}

fn render(el: ElementRef<'_>, walker: &mut Walker) {
    walker.children(el);
}

/// Stack Exchange pages: the question, then answers by score.
fn question_and_answers(root: ElementRef<'_>, title: &str, walker: &mut Walker) -> bool {
    let mut answers: Vec<(i64, bool, ElementRef<'_>)> = descendants(root)
        .filter(|e| has_class(e, "answer") && attr(e, "data-score").is_some())
        .map(|e| {
            let score = attr(&e, "data-score")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            (score, has_class(&e, "accepted-answer"), e)
        })
        .collect();
    if answers.is_empty() {
        return false;
    }
    answers.sort_by_key(|(score, _, _)| std::cmp::Reverse(*score));
    if !title.is_empty() {
        walker.block_break();
        walker.push_inline(&format!("# {title}"));
    }
    if let Some(question) = find_first(root, |e| has_class(e, "question")) {
        walker.block_break();
        walker.push_inline("## Question");
        walker.block_break();
        render(post_body(question), walker);
    }
    for (score, accepted, answer) in answers {
        walker.block_break();
        let mark = if accepted { ", accepted" } else { "" };
        walker.push_inline(&format!("## Answer (score {score}{mark})"));
        walker.block_break();
        render(post_body(answer), walker);
    }
    true
}

/// Extracts the readable text of an HTML page.
///
/// The page's `<main>` or `<article>` is used when it has one, otherwise its
/// `<body>`. Link targets are dropped: the text is data, and nothing in it is
/// followed.
#[must_use]
pub fn extract_main_text(html: &str) -> Extracted {
    let document = Html::parse_document(html);
    let root = document.root_element();
    let mut title = find_first(root, |e| e.value().name() == "title")
        .map(|e| {
            e.text()
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    if title.is_empty()
        && let Some(h1) = find_first(root, |e| e.value().name() == "h1")
    {
        title = h1
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    }

    let mut walker = Walker::default();
    if !question_and_answers(root, &title, &mut walker) {
        let content = find_first(root, |e| e.value().name() == "main")
            .or_else(|| find_first(root, |e| e.value().name() == "article"))
            .or_else(|| find_first(root, |e| e.value().name() == "body"))
            .unwrap_or(root);
        walker.children(content);
    }
    let math = if walker.tex {
        MathKind::Tex
    } else if walker.rendered {
        MathKind::Rendered
    } else {
        MathKind::None
    };
    Extracted {
        title,
        text: walker.finish(),
        math,
    }
}

#[cfg(test)]
mod test;
