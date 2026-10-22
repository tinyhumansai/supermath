//! Tests for the shared tool helpers and the tools' metadata.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use serde_json::json;
use supermath_files::Workspace;
use supermath_tool::{Budget, Exposure, Kind, Permission};

use super::{Python, Shared, contains_float, parse_args};
use crate::{Error, PythonConfig};

fn shared(dir: &std::path::Path) -> Shared {
    let python = Python::new(&PythonConfig::default(), Arc::new(Workspace::new(dir))).unwrap();
    Arc::try_unwrap(python.shared).unwrap()
}

#[test]
fn the_toolkit_hands_out_three_primitive_read_tools() {
    let dir = tempfile::tempdir().unwrap();
    let python = Python::new(
        &PythonConfig::default(),
        Arc::new(Workspace::new(dir.path())),
    )
    .unwrap();
    let tools = python.tools();
    let facts: Vec<_> = tools
        .iter()
        .map(|t| (t.name(), t.exposure(), t.permission(), t.kind()))
        .collect();
    assert_eq!(
        facts,
        [
            (
                "py_run",
                Exposure::Direct,
                Permission::Read,
                Kind::Primitive
            ),
            (
                "py_search_counterexample",
                Exposure::Deferred,
                Permission::Read,
                Kind::Primitive
            ),
            (
                "py_solvers",
                Exposure::Deferred,
                Permission::Read,
                Kind::Primitive
            ),
        ]
    );
    assert!(!python.client().is_configured());
    assert!(Arc::ptr_eq(&python.observer(), &python.observer()));
}

#[test]
fn schemas_are_closed_objects() {
    let dir = tempfile::tempdir().unwrap();
    let python = Python::new(
        &PythonConfig::default(),
        Arc::new(Workspace::new(dir.path())),
    )
    .unwrap();
    for tool in python.tools() {
        let schema = tool.schema();
        assert_eq!(
            schema["additionalProperties"],
            json!(false),
            "{}",
            tool.name()
        );
        assert_eq!(schema["type"], json!("object"));
    }
}

#[test]
fn an_unnamed_timeout_is_the_default_shortened_to_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(dir.path());
    let roomy = Budget {
        wall_time_ms: 600_000,
        ..Budget::default()
    };
    assert_eq!(shared.timeout(&roomy, None).unwrap(), 60);
    let tight = Budget {
        wall_time_ms: 1_500,
        ..Budget::default()
    };
    assert_eq!(shared.timeout(&tight, None).unwrap(), 2);
    let sliver = Budget {
        wall_time_ms: 300,
        ..Budget::default()
    };
    assert_eq!(shared.timeout(&sliver, None).unwrap(), 1);
}

#[test]
fn a_spent_compute_budget_is_refused_with_or_without_a_named_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(dir.path());
    let spent = Budget {
        wall_time_ms: 0,
        ..Budget::default()
    };
    for requested in [None, Some(1), Some(60)] {
        assert!(
            matches!(
                shared.timeout(&spent, requested),
                Err(Error::BudgetExceeded(_))
            ),
            "{requested:?}"
        );
    }
}

#[test]
fn a_named_timeout_is_checked_against_the_cap_and_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(dir.path());
    let roomy = Budget {
        wall_time_ms: 2_000_000,
        ..Budget::default()
    };
    assert_eq!(shared.timeout(&roomy, Some(120)).unwrap(), 120);
    assert!(matches!(
        shared.timeout(&roomy, Some(901)),
        Err(Error::BudgetExceeded(_))
    ));
    assert!(matches!(
        shared.timeout(&roomy, Some(0)),
        Err(Error::InvalidInput(_))
    ));
    let tight = Budget {
        wall_time_ms: 10_000,
        ..Budget::default()
    };
    assert!(matches!(
        shared.timeout(&tight, Some(11)),
        Err(Error::BudgetExceeded(_))
    ));
    assert_eq!(shared.timeout(&tight, Some(10)).unwrap(), 10);
}

#[test]
fn prepare_refuses_lean_paths_bad_paths_and_missing_files() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(dir.path());
    let lean = shared.prepare("demo", "lean/Supermath/A.lean").unwrap_err();
    assert!(matches!(lean, Error::InvalidInput(_)), "{lean}");
    let bad = shared.prepare("demo", "../x.py").unwrap_err();
    assert_eq!(bad.code(), supermath_tool::ErrorCode::InvalidInput);
    let missing = shared.prepare("demo", "python/work/absent.py").unwrap_err();
    assert_eq!(missing.code(), supermath_tool::ErrorCode::NotFound);
}

#[test]
fn floats_are_found_at_any_depth_and_integers_are_not() {
    assert!(contains_float(&json!({"a": [1, {"b": 2.5}]})));
    assert!(!contains_float(
        &json!({"a": [1, {"b": 2}], "c": "1.5", "d": null, "e": true})
    ));
}

#[test]
fn bad_argument_shapes_are_invalid_input() {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        #[allow(dead_code)]
        n: u64,
    }
    assert!(matches!(
        parse_args::<Args>(json!({"n": "x"})),
        Err(Error::InvalidInput(_))
    ));
    assert!(matches!(
        parse_args::<Args>(json!({"n": 1, "extra": 1})),
        Err(Error::InvalidInput(_))
    ));
}
