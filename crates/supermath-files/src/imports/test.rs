//! Import parsing and the allow-lists, per language.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Import, check, is_project_module, parse, strip_lean_comments};
use crate::error::Error;
use crate::path::Language;

fn modules(language: Language, source: &str) -> Vec<String> {
    parse(language, source)
        .into_iter()
        .map(|i| i.module)
        .collect()
}

#[test]
fn lean_header_lists_imports_through_comments_and_modifiers() {
    let source = "/- header /- nested -/ import Evil -/\nprelude\nimport Mathlib -- import Std\n\
                  public import Mathlib.Tactic import Supermath.A.B\n\ntheorem t : True := trivial\n\
                  import Lean\n";
    assert_eq!(
        modules(Language::Lean, source),
        ["Mathlib", "Mathlib.Tactic", "Supermath.A.B"]
    );
}

#[test]
fn lean_header_tolerates_a_dangling_import_and_stray_modifier() {
    assert_eq!(modules(Language::Lean, "import A\nimport"), ["A"]);
    assert_eq!(
        modules(Language::Lean, "private theorem x := 1"),
        Vec::<String>::new()
    );
}

#[test]
fn lean_comment_stripping_keeps_line_numbers() {
    let cleaned = strip_lean_comments("a -- b\n/- c\nd -/ e\nf");
    assert_eq!(cleaned, "a \n\n e\nf");
    assert_eq!(strip_lean_comments("x /- unclosed"), "x ");
}

#[test]
fn lean_allows_mathlib_and_supermath_only() {
    assert!(
        check(
            Language::Lean,
            "import Mathlib\nimport Mathlib.Data.Nat\nimport Supermath.X"
        )
        .is_ok()
    );
    for source in [
        "import Lean",
        "import Std",
        "import Mathlibrary",
        "import Supermath",
        "import Init",
    ] {
        let error = check(Language::Lean, source).unwrap_err();
        assert!(
            matches!(
                error,
                Error::ForbiddenImport {
                    language: Language::Lean,
                    ..
                }
            ),
            "{source}"
        );
    }
}

#[test]
fn python_statements_are_found_at_any_depth() {
    let source = "import os, sys as s\nfrom work.a import b, c as d\nfrom work import (\n    x,\n    y as z,\n)\n\
                  def f():\n    import numpy; import sympy\n    return 1  # import requests\n";
    let found = parse(Language::Python, source);
    let rendered: Vec<(String, Vec<String>)> = found
        .into_iter()
        .map(|Import { module, names }| (module, names))
        .collect();
    let owned = |m: &str, n: &[&str]| {
        (
            m.to_owned(),
            n.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        )
    };
    assert_eq!(
        rendered,
        [
            owned("os", &[]),
            owned("sys", &[]),
            owned("work.a", &["b", "c"]),
            owned("work", &["x", "y"]),
            owned("numpy", &[]),
            owned("sympy", &[]),
        ]
    );
}

#[test]
fn python_ignores_imports_inside_triple_quoted_strings() {
    let source = "\"\"\"\nimport requests\nfrom nowhere import x\n\"\"\"\nx = '''\nimport evil\n'''\nimport math\n\
                  \"\"\"one line import fake\"\"\"\nfrom the above we see\nfrom 1bad! import y\n";
    assert_eq!(modules(Language::Python, source), ["math"]);
}

#[test]
fn python_from_import_star_and_garbage_names_are_dropped() {
    let found = parse(
        Language::Python,
        "from work.a import *\nfrom work.b import c.d, e",
    );
    assert_eq!(found[0].names, Vec::<String>::new());
    assert_eq!(found[1].names, ["e"]);
}

#[test]
fn python_allows_stdlib_solvers_and_work() {
    let source = "import math\nimport numpy.linalg\nfrom sympy import symbols\nimport z3\nimport work\nfrom work.a import b";
    assert!(check(Language::Python, source).is_ok());
}

#[test]
fn python_refuses_foreign_and_relative_imports() {
    for source in [
        "import requests",
        "from pandas import DataFrame",
        "import workaround",
        "from . import a",
        "from .a import b",
    ] {
        let error = check(Language::Python, source).unwrap_err();
        assert!(
            matches!(
                error,
                Error::ForbiddenImport {
                    language: Language::Python,
                    ..
                }
            ),
            "{source}"
        );
    }
}

#[test]
fn project_modules_are_recognised_per_language() {
    assert!(is_project_module(Language::Lean, "Supermath.A"));
    assert!(!is_project_module(Language::Lean, "Supermath"));
    assert!(is_project_module(Language::Python, "work"));
    assert!(is_project_module(Language::Python, "work.a"));
    assert!(!is_project_module(Language::Python, "workaround"));
}
