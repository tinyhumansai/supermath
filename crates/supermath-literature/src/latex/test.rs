//! Tests for flattening, the section outline, and the scanners.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::scan::{brace_arg, bracket_arg, command_args, control_word, read_group, strip_comments};
use super::{SourceTree, flatten_latex, outline};

fn tree(files: &[(&str, &str)]) -> SourceTree {
    let mut tree = SourceTree::new();
    for (path, text) in files {
        tree.insert(path, (*text).to_owned());
    }
    tree
}

fn flat(source: &str) -> String {
    flatten_latex(&tree(&[("main.tex", source)])).text
}

// --- scanners ---------------------------------------------------------------

#[test]
fn comments_are_removed_but_escaped_percent_signs_stay() {
    assert_eq!(strip_comments("a % b\nc"), "a \nc");
    assert_eq!(strip_comments("50\\% of x"), "50\\% of x");
    assert_eq!(strip_comments("line\\\\% comment\nnext"), "line\\\\\nnext");
    assert_eq!(strip_comments("% whole line\nx"), "\nx");
}

#[test]
fn groups_nest_and_skip_escaped_braces() {
    assert_eq!(read_group("{a{b}c}d", 0, b'{', b'}'), Some(("a{b}c", 7)));
    assert_eq!(read_group("{a\\}b}", 0, b'{', b'}'), Some(("a\\}b", 6)));
    assert_eq!(read_group("{open", 0, b'{', b'}'), None);
    assert_eq!(read_group("x{a}", 0, b'{', b'}'), None);
    assert_eq!(brace_arg("  {x}", 0), Some(("x", 5)));
    assert_eq!(bracket_arg(" [y] z", 0), Some(("y", 4)));
}

#[test]
fn control_words_and_symbols_are_told_apart() {
    assert_eq!(control_word("\\begin{x}", 0), ("begin", 6));
    assert_eq!(control_word("\\\\x", 0), ("", 1));
    assert_eq!(
        command_args(
            "\\ref{a} \\\\ \\eqref{b} \\label*{c} \\other{d}",
            &["ref", "eqref", "label"]
        ),
        ["a", "b", "c"]
    );
}

// --- the tree -----------------------------------------------------------------

#[test]
fn paths_are_normalised_and_inputs_resolve_relative_then_from_the_root() {
    let t = tree(&[
        ("./sec/a.tex", "A"),
        ("sec/../b.tex", "B"),
        ("sec/deep/c.tex", "C"),
    ]);
    assert_eq!(t.paths(), ["b.tex", "sec/a.tex", "sec/deep/c.tex"]);
    assert_eq!(t.resolve("a", "sec/main.tex").as_deref(), Some("sec/a.tex"));
    assert_eq!(t.resolve("b.tex", "sec/main.tex").as_deref(), Some("b.tex"));
    assert_eq!(
        t.resolve("deep/c", "sec/x.tex").as_deref(),
        Some("sec/deep/c.tex")
    );
    assert_eq!(t.resolve("zzz", "sec/x.tex"), None);
    assert!(!t.is_empty() && SourceTree::new().is_empty());
}

#[test]
fn the_main_file_is_the_one_with_the_document() {
    let t = tree(&[
        ("a.tex", "\\section{only a part}"),
        (
            "paper.tex",
            "\\documentclass{x}\\begin{document}\\end{document}",
        ),
        ("zz.tex", "\\documentclass{x}"),
    ]);
    assert_eq!(t.main_file(), Some("paper.tex"));
    assert_eq!(SourceTree::new().main_file(), None);
    // With nothing distinguishing them, the largest file wins.
    let t = tree(&[("a.tex", "short"), ("b.tex", "much longer text")]);
    assert_eq!(t.main_file(), Some("b.tex"));
}

// --- flattening -------------------------------------------------------------------

#[test]
fn input_and_include_are_inlined_recursively() {
    let t = tree(&[
        (
            "main.tex",
            "\\documentclass{x}\\begin{document}A \\input{one} B \\include{two}\\end{document}",
        ),
        ("one.tex", "one \\input{sub/three}"),
        ("two.tex", "two"),
        ("sub/three.tex", "three % gone"),
    ]);
    let out = flatten_latex(&t);
    assert_eq!(out.main, "main.tex");
    assert!(out.unknowns.is_empty(), "{:?}", out.unknowns);
    let words: Vec<&str> = out.text.split_whitespace().collect();
    assert_eq!(
        words,
        [
            "\\documentclass{x}\\begin{document}A",
            "one",
            "three",
            "B",
            "two",
            "\\end{document}"
        ]
    );
}

#[test]
fn a_missing_input_and_a_cycle_are_unknowns_not_failures() {
    let t = tree(&[
        (
            "main.tex",
            "\\documentclass{x}\\input{gone}\\input{loop}\\begin{document}",
        ),
        ("loop.tex", "x\\input{loop}y"),
    ]);
    let out = flatten_latex(&t);
    assert_eq!(out.unknowns.len(), 2);
    assert!(out.unknowns[0].contains("gone"));
    assert!(out.unknowns[1].contains("cycle"));
    assert!(out.text.contains("xy"));
}

#[test]
fn input_nesting_is_capped() {
    let mut files: Vec<(String, String)> = (0..20)
        .map(|i| (format!("f{i}.tex"), format!("\\input{{f{}}}", i + 1)))
        .collect();
    files.push(("f20.tex".into(), "end".into()));
    files.insert(
        0,
        (
            "main.tex".into(),
            "\\documentclass{x}\\begin{document}\\input{f0}".into(),
        ),
    );
    let mut t = SourceTree::new();
    for (p, s) in &files {
        t.insert(p, s.clone());
    }
    let out = flatten_latex(&t);
    assert!(
        out.unknowns.iter().any(|u| u.contains("deeper than")),
        "{:?}",
        out.unknowns
    );
    assert!(!out.text.contains("end"));
}

#[test]
fn an_empty_source_flattens_to_nothing_with_an_unknown() {
    let out = flatten_latex(&SourceTree::new());
    assert_eq!(out.text, "");
    assert_eq!(out.unknowns, ["the source has no .tex files"]);
}

#[test]
fn argument_free_macros_are_expanded_and_their_definitions_removed() {
    let source = "\\newcommand{\\R}{\\mathbb{R}}\n\\newcommand*\\N{\\mathbb N}\n\\providecommand{\\Z}{\\mathbb{Z}}\n\
        \\def\\Q{\\mathbb{Q}}\n\\DeclareMathOperator{\\rad}{rad}\n\\DeclareMathOperator*{\\argmax}{arg\\,max}\n\
        \\newcommand{\\RN}{\\R^n}\n$\\R \\N \\Z \\Q \\rad \\argmax \\RN$ \\\\ 100\\%";
    let out = flatten_latex(&tree(&[("main.tex", source)]));
    assert_eq!(out.macros, 7);
    let body = out.text.trim();
    assert_eq!(
        body,
        "$\\mathbb{R} \\mathbb N \\mathbb{Z} \\mathbb{Q} \\operatorname{rad} \\operatorname{arg\\,max} \\mathbb{R}^n$ \\\\ 100\\%"
    );
}

#[test]
fn macros_with_arguments_renewcommand_and_counter_names_are_left_alone() {
    let source = "\\newcommand{\\abs}[1]{|#1|}\\newcommand{\\opt}[2][x]{y}\\renewcommand{\\vec}{v}\n\
        \\def\\thetheorem{\\thesection.\\arabic{theorem}}\\def\\f#1{#1}\\newcommand{x}{bad}\\newcommand{\\a1}{bad}\\def\\g{unbraced\n\
        \\abs{x} \\vec \\thetheorem \\def";
    let text = flat(source);
    for kept in [
        "\\newcommand{\\abs}[1]",
        "\\renewcommand{\\vec}{v}",
        "\\def\\thetheorem",
        "\\def\\f#1",
        "\\abs{x}",
        "\\def\\g{unbraced",
    ] {
        assert!(text.contains(kept), "{kept} missing from {text}");
    }
}

#[test]
fn macros_expand_through_each_other_but_not_forever() {
    let text = flat(
        "\\newcommand{\\a}{\\b}\\newcommand{\\b}{\\c}\\newcommand{\\c}{z}\\newcommand{\\loop}{\\loop!}\\a \\loop",
    );
    assert!(text.trim().starts_with('z'));
    assert!(text.contains("\\loop!"));
}

// --- the outline ------------------------------------------------------------------

#[test]
fn headings_are_numbered_like_latex() {
    let text = "\\section{A}\\subsection{B}\\subsubsection{C}\\subsection{D}\\section*{E}\\section[short]{F \\label{x}}\
        \\appendix\\section{G}\\subsection{H}\\section{I}\\end{document}\\section{after}";
    let o = outline(text);
    let got: Vec<(u8, Option<&str>, &str)> = o
        .headings
        .iter()
        .map(|h| (h.level, h.number.as_deref(), h.title.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            (1, Some("1"), "A"),
            (2, Some("1.1"), "B"),
            (3, Some("1.1.1"), "C"),
            (2, Some("1.2"), "D"),
            (1, None, "E"),
            (1, Some("2"), "F"),
            (1, Some("A"), "G"),
            (2, Some("A.1"), "H"),
            (1, Some("B"), "I"),
        ]
    );
    assert_eq!(o.end, text.find("\\end{document}").unwrap());
}

#[test]
fn section_text_runs_to_the_next_heading_of_the_same_or_higher_level() {
    let text = "pre \\section{A} alpha \\subsection{B} beta \\subsection{C} gamma \\section{D} delta \\end{document} tail";
    let o = outline(text);
    assert_eq!(
        o.section_text(text, 0),
        "alpha \\subsection{B} beta \\subsection{C} gamma"
    );
    assert_eq!(o.section_text(text, 1), "beta");
    assert_eq!(o.section_text(text, 3), "delta");
    assert_eq!(o.section_text(text, 9), "");
    assert_eq!(o.find("1.2"), Some(2));
    assert_eq!(o.find("7"), None);
}

#[test]
fn the_current_heading_is_the_last_one_before_a_position() {
    let text = "\\section{A} x \\subsection{B} y";
    let o = outline(text);
    assert!(o.current(0).is_none());
    assert_eq!(o.current(text.len()).unwrap().title, "B");
    assert_eq!(o.current(text.find('x').unwrap()).unwrap().title, "A");
}

#[test]
fn malformed_headings_are_skipped() {
    let o = outline("\\section \\section* \\subsection[x] \\sectionmark{y} \\appendix");
    assert_eq!(o.headings.len(), 0);
}
