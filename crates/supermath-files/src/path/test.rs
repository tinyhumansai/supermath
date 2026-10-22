//! Path rules: a test per refusal and per acceptance.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Language, MAX_PATH_BYTES, ProjectPath};

#[test]
fn accepts_lean_and_python_module_paths() {
    for raw in [
        "lean/Supermath/Sendov.lean",
        "lean/Supermath/Sendov/Main.lean",
        "lean/Supermath/A1/B2c.lean",
        "python/work/sendov_search.py",
        "python/work/a/b_2/c.py",
    ] {
        let path = ProjectPath::parse(raw).unwrap();
        assert_eq!(path.as_str(), raw);
        assert_eq!(path.to_string(), raw);
    }
}

#[test]
fn refuses_every_path_rule_violation() {
    let long = format!("python/work/{}.py", "a".repeat(MAX_PATH_BYTES));
    let bad = [
        "",
        "/lean/Supermath/A.lean",
        "lean\\Supermath\\A.lean",
        "C:/lean/Supermath/A.lean",
        "lean/Supermath/../A.lean",
        "lean/Supermath/./A.lean",
        "lean//Supermath/A.lean",
        "lean/Supermath/A.lean/",
        "rust/Supermath/A.lean",
        "lean/Other/A.lean",
        "lean/Supermath",
        "lean/Supermath/A.py",
        "lean/Supermath/a.lean",
        "lean/Supermath/A_b.lean",
        "lean/Supermath/.lean",
        "lean/Supermath/Bad dir/A.lean",
        "python/work/Upper.py",
        "python/work/_private.py",
        "python/work/1st.py",
        "python/work/a-b.py",
        "python/work/__init__.py",
        "python/work/a.lean",
        "python/other/a.py",
        long.as_str(),
    ];
    for raw in bad {
        assert!(
            ProjectPath::parse(raw).is_err(),
            "{raw:?} should be refused"
        );
    }
}

#[test]
fn module_and_from_module_round_trip() {
    let lean = ProjectPath::parse("lean/Supermath/Sendov/Main.lean").unwrap();
    assert_eq!(lean.module(), "Supermath.Sendov.Main");
    assert_eq!(
        ProjectPath::from_module(Language::Lean, "Supermath.Sendov.Main"),
        Some(lean)
    );
    let py = ProjectPath::parse("python/work/sendov_search.py").unwrap();
    assert_eq!(py.module(), "work.sendov_search");
    assert_eq!(
        ProjectPath::from_module(Language::Python, "work.sendov_search"),
        Some(py)
    );
}

#[test]
fn from_module_refuses_foreign_or_malformed_names() {
    assert!(ProjectPath::from_module(Language::Lean, "Mathlib.Tactic").is_none());
    assert!(ProjectPath::from_module(Language::Lean, "Supermath").is_none());
    assert!(ProjectPath::from_module(Language::Lean, "Supermath..A").is_none());
    assert!(ProjectPath::from_module(Language::Python, "numpy.linalg").is_none());
    assert!(ProjectPath::from_module(Language::Python, "work.Bad").is_none());
}

#[test]
fn generated_directories_are_recognised() {
    let witness = ProjectPath::parse("lean/Supermath/Witnesses/W1.lean").unwrap();
    assert!(witness.is_generated());
    assert!(
        ProjectPath::parse("python/work/witnesses/w1.py")
            .unwrap()
            .is_generated()
    );
    assert!(
        !ProjectPath::parse("lean/Supermath/Sendov/Main.lean")
            .unwrap()
            .is_generated()
    );
    // A file named like the directory is not inside it.
    assert!(
        !ProjectPath::parse("lean/Supermath/Witnesses.lean")
            .unwrap()
            .is_generated()
    );
}

#[test]
fn language_exposes_its_layout() {
    assert_eq!(Language::Lean.root_dir(), "lean");
    assert_eq!(Language::Python.extension(), "py");
    assert_eq!(Language::Lean.package_dir(), "Supermath");
    assert_eq!(Language::Python.generated_dir(), "witnesses");
    assert_eq!(Language::Python.to_string(), "python");
    assert_eq!(serde_json::to_string(&Language::Lean).unwrap(), "\"lean\"");
}
