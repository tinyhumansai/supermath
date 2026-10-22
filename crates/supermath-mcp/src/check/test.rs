//! The report's verdict and rendering. `tests/check.rs` probes stubs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Line, Report, Status};

fn report(statuses: &[Status]) -> Report {
    Report {
        lines: statuses
            .iter()
            .map(|status| Line::new("lean", *status, "detail"))
            .collect(),
    }
}

#[test]
fn skipped_backends_do_not_fail_the_report() {
    assert!(report(&[Status::Ok, Status::Skipped]).passed());
    assert!(report(&[]).passed());
}

#[test]
fn one_failed_backend_fails_the_report() {
    assert!(!report(&[Status::Ok, Status::Failed]).passed());
}

#[test]
fn each_line_names_its_backend_and_status() {
    let shown = Report {
        lines: vec![
            Line::new("lean", Status::Ok, "toolchain v4"),
            Line::new("python", Status::Skipped, "PYTHON_URL is not set"),
            Line::new("arxiv", Status::Failed, "timed out"),
        ],
    }
    .to_string();
    assert!(shown.contains("ok      lean: toolchain v4"), "{shown}");
    assert!(
        shown.contains("skipped python: PYTHON_URL is not set"),
        "{shown}"
    );
    assert!(shown.contains("FAILED  arxiv: timed out"), "{shown}");
}
