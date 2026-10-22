//! Tests for statement extraction: declarations, counters, labels, refs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Statement, extract_statements};

fn numbers(text: &str) -> Vec<(String, Option<String>)> {
    extract_statements(text)
        .statements
        .into_iter()
        .map(|s| (s.env, s.number))
        .collect()
}

fn pair(env: &str, number: Option<&str>) -> (String, Option<String>) {
    (env.to_owned(), number.map(str::to_owned))
}

const PREAMBLE: &str = "\\newtheorem{thm}{Theorem}[section]\n\\newtheorem{lem}[thm]{Lemma}\n\
    \\newtheorem{defn}{Definition}\n\\newtheorem*{rmk}{Remark}\n\\newtheorem{conj}{Conjecture}";

#[test]
fn shared_counters_within_section_and_independent_counters_number_like_latex() {
    let body = "\\section{A}\n\
        \\begin{thm}x\\end{thm}\\begin{lem}x\\end{lem}\\begin{defn}x\\end{defn}\n\
        \\section{B}\n\
        \\begin{lem}x\\end{lem}\\begin{rmk}x\\end{rmk}\\begin{thm}x\\end{thm}\\begin{defn}x\\end{defn}\\begin{conj}x\\end{conj}";
    assert_eq!(
        numbers(&format!("{PREAMBLE}\n{body}")),
        [
            pair("thm", Some("1.1")),
            pair("lem", Some("1.2")),
            pair("defn", Some("1")),
            pair("lem", Some("2.1")),
            pair("rmk", None),
            pair("thm", Some("2.2")),
            pair("defn", Some("2")),
            pair("conj", Some("1")),
        ]
    );
}

#[test]
fn subsections_do_not_reset_a_within_section_counter_but_do_for_within_subsection() {
    let text = "\\newtheorem{a}{A}[section]\\newtheorem{b}{B}[subsection]\\section{S}\\subsection{T}\
        \\begin{a}x\\end{a}\\begin{b}x\\end{b}\\subsection{U}\\begin{a}x\\end{a}\\begin{b}x\\end{b}\
        \\section{V}\\begin{b}x\\end{b}\\subsection*{W}\\begin{b}x\\end{b}";
    assert_eq!(
        numbers(text),
        [
            pair("a", Some("1.1")),
            pair("b", Some("1.1.1")),
            pair("a", Some("1.2")),
            pair("b", Some("1.2.1")),
            pair("b", Some("2.0.1")),
            pair("b", Some("2.0.2")),
        ]
    );
}

#[test]
fn statements_before_any_section_use_section_zero() {
    let text = "\\newtheorem{a}{A}[section]\\newtheorem{b}{B}[subsection]\\begin{a}x\\end{a}\\begin{b}x\\end{b}";
    assert_eq!(
        numbers(text),
        [pair("a", Some("0.1")), pair("b", Some("0.1"))]
    );
}

#[test]
fn appendix_sections_number_with_letters() {
    let text = "\\newtheorem{a}{A}[section]\\section{S}\\begin{a}x\\end{a}\\appendix\\section{T}\\begin{a}x\\end{a}";
    assert_eq!(
        numbers(text),
        [pair("a", Some("1.1")), pair("a", Some("A.1"))]
    );
}

#[test]
fn a_chain_of_shared_counters_follows_to_the_owner() {
    let text = "\\newtheorem{t}{T}\\newtheorem{l}[t]{L}\\newtheorem{c}[l]{C}\\begin{t}x\\end{t}\\begin{c}x\\end{c}\\begin{l}x\\end{l}";
    assert_eq!(
        numbers(text),
        [
            pair("t", Some("1")),
            pair("c", Some("2")),
            pair("l", Some("3"))
        ]
    );
}

#[test]
fn a_statement_has_kind_label_section_text_and_refs() {
    let text = format!(
        "{PREAMBLE}\n\\section{{Intro}}\n\\begin{{thm}}[Big Name]\\label{{thm:big}}\n\
         Uses \\ref{{lem:a}}, \\cref{{x:1, x:2}} and \\eqref{{eq:1}}; again \\ref{{lem:a}} and itself \\ref{{thm:big}}.\n\\end{{thm}}\n\
         \\begin{{proof}} By \\autoref{{def:z}} and \\Cref{{x:1}} and \\vref{{v:1}}. \\end{{proof}}"
    );
    let found = extract_statements(&text);
    assert_eq!(found.unknowns.len(), 0);
    let Statement {
        label,
        kind,
        env,
        number,
        section,
        statement,
        refs,
    } = &found.statements[0];
    assert_eq!(label.as_deref(), Some("thm:big"));
    assert_eq!((kind.as_str(), env.as_str()), ("theorem", "thm"));
    assert_eq!(number.as_deref(), Some("1.1"));
    assert_eq!(section.as_deref(), Some("1"));
    assert!(statement.starts_with("Uses \\ref{lem:a}") && !statement.contains("\\label"));
    assert_eq!(refs, &["lem:a", "x:1", "x:2", "eq:1", "def:z", "v:1"]);
}

#[test]
fn a_proof_titled_for_another_statement_is_not_attached() {
    let text = "\\newtheorem{t}{T}\\begin{t}\\label{a}one\\end{t}\\begin{proof}[Proof of \\ref{b}]\\ref{c}\\end{proof}\
        \\begin{t}\\label{b}two\\end{t}\\begin{proof}[Proof of \\ref{b}]\\ref{d}\\end{proof}\
        \\begin{t}three\\end{t}\\begin{proof}[Sketch]\\ref{e}\\end{proof}\
        \\begin{t}four\\end{t} text \\begin{proof}\\ref{f}\\end{proof}\
        \\begin{t}five\\end{t}\\begin{proof}never closed";
    let refs: Vec<Vec<String>> = extract_statements(text)
        .statements
        .into_iter()
        .map(|s| s.refs)
        .collect();
    assert_eq!(refs, [vec![], vec!["d"], vec!["e"], vec![], vec![]]);
}

#[test]
fn an_unlabelled_statement_has_no_label_and_keeps_its_number() {
    let found = extract_statements("\\newtheorem{t}{Theorem}\\begin{t}x\\end{t}");
    assert_eq!(found.statements[0].label, None);
    assert_eq!(found.statements[0].number.as_deref(), Some("1"));
}

#[test]
fn an_optional_argument_that_swallows_the_body_is_not_a_name() {
    let found = extract_statements("\\newtheorem{t}{T}\\begin{t}[unclosed text\\end{t}");
    assert_eq!(found.statements[0].statement, "[unclosed text");
}

#[test]
fn undeclared_standard_environments_are_reported_not_guessed() {
    let text = "\\newtheorem{theorem}{Theorem}\\begin{theorem}ok\\end{theorem}\\begin{lemma}a\\end{lemma}\\begin{lemma}b\\end{lemma}\\begin{figure}f\\end{figure}";
    let found = extract_statements(text);
    assert_eq!(found.statements.len(), 1);
    assert_eq!(found.unknowns.len(), 1);
    assert!(found.unknowns[0].starts_with("2 \\begin{lemma}"));
}

#[test]
fn a_redefined_counter_leaves_its_statements_unnumbered_and_listed() {
    let text = "\\newtheorem{t}{T}\\newtheorem{u}[t]{U}\\newtheorem{v}{V}\\renewcommand{\\thet}{\\Roman{t}}\\begin{t}a\\end{t}\\begin{u}b\\end{u}\\begin{v}c\\end{v}";
    let found = extract_statements(text);
    assert_eq!(
        found
            .statements
            .iter()
            .map(|s| s.env.as_str())
            .collect::<Vec<_>>(),
        ["v"]
    );
    assert_eq!(found.unknowns.len(), 1);
    assert!(found.unknowns[0].contains("counter t"));
    for def in ["\\renewcommand\\thet{x}", "\\def\\thet{x}"] {
        let text = format!("\\newtheorem{{t}}{{T}}{def}\\begin{{t}}a\\end{{t}}");
        assert_eq!(extract_statements(&text).statements.len(), 0);
    }
}

#[test]
fn unsupported_numbering_is_reported() {
    let within = extract_statements("\\newtheorem{t}{T}[chapter]\\begin{t}a\\end{t}");
    assert_eq!(within.statements.len(), 0);
    assert!(within.unknowns[0].contains("chapter"));
    let shared = extract_statements("\\newtheorem{t}[section]{T}\\begin{t}a\\end{t}");
    assert_eq!(shared.statements.len(), 0);
    assert!(shared.unknowns[0].contains("not a theorem environment"));
    let cycle = extract_statements("\\newtheorem{a}[b]{A}\\newtheorem{b}[a]{B}\\begin{a}x\\end{a}");
    assert_eq!(cycle.statements.len(), 0);
    assert_eq!(cycle.unknowns.len(), 1);
}

#[test]
fn an_unclosed_environment_is_reported() {
    let found = extract_statements("\\newtheorem{t}{T}\\begin{t}never ends");
    assert_eq!(found.statements.len(), 0);
    assert!(found.unknowns[0].contains("no \\end"));
}

#[test]
fn malformed_declarations_and_begins_are_ignored() {
    let text = "\\newtheorem \\newtheorem{only} \\newtheorem*{s}{S} \\begin \\begin{t}x\\end{t} \\begin{s}y\\end{s}";
    assert_eq!(numbers(text), [pair("s", None)]);
}

#[test]
fn the_title_is_the_kind_lowercased_with_whitespace_collapsed() {
    let found = extract_statements("\\newtheorem{mt}{Main   Theorem}\\begin{mt}x\\end{mt}");
    assert_eq!(found.statements[0].kind, "main theorem");
}

#[test]
fn lyx_protected_theorem_names_read_as_their_kind() {
    // LyX writes this preamble (arXiv 1505.05143): titles are `\protect`ed
    // name macros defined after the declarations.
    let source = "\\newtheorem{thm}{\\protect\\theoremname}[section]\n\
        \\newtheorem{question}[thm]{\\protect\\questionname}\n\
        \\newtheorem{lem}[thm]{\\protect\\lemmaname}\n\
        \\providecommand{\\lemmaname}{Lemma}\n\
        \\providecommand{\\questionname}{Question}\n\
        \\providecommand{\\theoremname}{Theorem}\n\
        \\begin{document}\\section{A}\n\
        \\begin{thm}x\\end{thm}\\begin{question}y\\end{question}\\begin{lem}z\\end{lem}";
    let mut tree = crate::SourceTree::new();
    tree.insert("main.tex", source.to_owned());
    let flat = crate::flatten_latex(&tree);
    let kinds: Vec<String> = extract_statements(&flat.text)
        .statements
        .into_iter()
        .map(|s| s.kind)
        .collect();
    assert_eq!(kinds, ["theorem", "question", "lemma"]);
}
