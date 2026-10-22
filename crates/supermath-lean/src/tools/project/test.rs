//! Unit tests for the dependency-ordered file list.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use supermath_files::{ProjectPath, Workspace, WriteMeta};

use super::lean_files;

fn put(workspace: &Workspace, path: &str, text: &str) {
    let path = ProjectPath::parse(path).unwrap();
    workspace
        .write(
            "demo",
            &path,
            text,
            None,
            &WriteMeta::new("file_write", "c").allowing_generated(),
        )
        .unwrap();
}

#[test]
fn imports_come_before_importers() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(dir.path());
    put(
        &workspace,
        "lean/Supermath/A.lean",
        "import Supermath.C\nimport Supermath.B\n",
    );
    put(&workspace, "lean/Supermath/B.lean", "import Mathlib\n");
    put(&workspace, "lean/Supermath/C.lean", "import Supermath.B\n");
    put(&workspace, "python/work/x.py", "import math\n");
    let order: Vec<String> = lean_files(&workspace, "demo")
        .unwrap()
        .into_iter()
        .map(|f| f.path.module())
        .collect();
    assert_eq!(order, ["Supermath.B", "Supermath.C", "Supermath.A"]);
}

#[test]
fn a_cycle_and_a_missing_import_do_not_loop_or_fail() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(dir.path());
    put(
        &workspace,
        "lean/Supermath/A.lean",
        "import Supermath.B\nimport Supermath.Gone\n",
    );
    put(&workspace, "lean/Supermath/B.lean", "import Supermath.A\n");
    let order = lean_files(&workspace, "demo").unwrap();
    assert_eq!(order.len(), 2);
}

#[test]
fn an_empty_project_has_no_files() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        lean_files(&Workspace::new(dir.path()), "demo")
            .unwrap()
            .is_empty()
    );
}
