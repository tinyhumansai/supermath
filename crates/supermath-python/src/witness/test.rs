//! Tests for the witness file's source and name.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use supermath_files::{ProjectPath, Workspace, WriteMeta};

use super::{path_for, render};
use crate::frame::Frame;

fn range() -> Frame {
    Frame::Range { from: 0, to: 100 }
}

#[test]
fn renders_a_runnable_module_that_rechecks_the_predicate() {
    let source = render("work.euler_poly", "is_prime_value", "41", &range());
    assert!(source.starts_with("\"\"\"A counterexample found"));
    assert!(source.contains("Predicate: work.euler_poly.is_prime_value"));
    assert!(source.contains("Frame: every integer from 0 to 100"));
    assert!(source.contains("from work.euler_poly import is_prime_value"));
    assert!(source.contains("WITNESS = json.loads(\"41\")"));
    assert!(source.contains("def check():"));
    assert!(source.contains("\"predicate_holds\": is_prime_value(WITNESS)"));
}

#[test]
fn a_product_witness_is_rebuilt_as_a_tuple() {
    let frame = Frame::Product {
        ranges: vec![(0, 3), (0, 3)],
    };
    let source = render("work.m", "p", "[1,2]", &frame);
    assert!(source.contains("WITNESS = tuple(json.loads(\"[1,2]\"))"));
}

#[test]
fn json_text_is_escaped_into_a_python_string() {
    let source = render("work.m", "p", r#"["a\"b\\n"]"#, &range());
    assert!(source.contains(r#"json.loads("[\"a\\\"b\\\\n\"]")"#));
}

#[test]
fn the_generated_file_passes_the_workspace_import_rules_and_write() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(dir.path());
    let source = render("work.euler_poly", "is_prime_value", "41", &range());
    let path = ProjectPath::parse(&path_for("is_prime_value", &source)).unwrap();
    assert!(path.is_generated());
    let meta = WriteMeta::new("py_search_counterexample", "call-1").allowing_generated();
    let written = workspace
        .write("demo", &path, &source, None, &meta)
        .unwrap();
    assert_eq!(workspace.sha256("demo", &path).unwrap(), written.sha256);
}

#[test]
fn the_name_is_content_addressed_and_always_a_valid_module_path() {
    let a = path_for("Is_Prime", "one");
    let b = path_for("Is_Prime", "two");
    assert_ne!(a, b);
    assert_eq!(a, path_for("Is_Prime", "one"));
    assert!(a.starts_with("python/work/witnesses/witness_is_prime_"));
    for name in [&a, &path_for("_x", "c"), &path_for("A1", "c")] {
        ProjectPath::parse(name).unwrap();
    }
}
