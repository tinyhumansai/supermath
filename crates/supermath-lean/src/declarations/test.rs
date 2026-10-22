//! Unit tests for the qualified-name scan.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::declarations;

fn names(source: &str) -> Vec<String> {
    declarations(source).into_iter().map(|d| d.name).collect()
}

#[test]
fn namespaces_qualify_and_end_restores_the_scope() {
    let source = "namespace A.B\ntheorem t : True := trivial\nend A.B\ndef top := 1\n";
    assert_eq!(names(source), ["A.B.t", "top"]);
}

#[test]
fn sections_do_not_qualify_but_are_closed_by_end() {
    let source = "section Foo\ndef a := 1\nend Foo\nnamespace N\ndef b := 2\nend N\n";
    assert_eq!(names(source), ["a", "N.b"]);
}

#[test]
fn mutual_blocks_are_closed_by_end() {
    let source = "namespace M\nmutual\ndef even := 1\ndef odd := 2\nend\ndef after := 3\nend M\n";
    assert_eq!(names(source), ["M.even", "M.odd", "M.after"]);
}

#[test]
fn root_names_ignore_the_namespace() {
    assert_eq!(
        names("namespace A\ntheorem _root_.B.t : True := trivial\nend A\n"),
        ["B.t"]
    );
}

#[test]
fn examples_and_anonymous_instances_are_skipped() {
    let source =
        "example : True := trivial\ninstance : Inhabited Nat := ⟨0⟩\ntheorem t : True := trivial\n";
    assert_eq!(names(source), ["t"]);
}

#[test]
fn comments_hide_commands_and_keep_line_numbers() {
    let source = "/- namespace Hidden\n-/\n-- def nope := 1\ntheorem t : True := by\n  sorry\n";
    let found = declarations(source);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "t");
    assert_eq!(found[0].line, 4);
    assert!(found[0].has_sorry);
    assert_eq!(found[0].kind, "theorem");
}

#[test]
fn a_bare_namespace_or_stray_end_is_harmless() {
    assert_eq!(
        names("namespace\ndef a := 1\nend\nend\ndef b := 2\n"),
        ["a", "b"]
    );
}
