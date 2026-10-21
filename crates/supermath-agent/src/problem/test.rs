//! Unit tests for problem loading and slug derivation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Problem, slugify};
use crate::error::Error;

#[test]
fn slugify_keeps_lowercase_letters_digits_and_single_dashes() {
    assert_eq!(
        slugify("Sum of Odd Numbers").as_deref(),
        Some("sum-of-odd-numbers")
    );
    assert_eq!(
        slugify("erdos_gyarfas--2").as_deref(),
        Some("erdos-gyarfas-2")
    );
    assert_eq!(slugify("  ../Collatz!  ").as_deref(), Some("collatz"));
}

#[test]
fn slugify_rejects_a_name_with_nothing_usable() {
    assert_eq!(slugify(""), None);
    assert_eq!(slugify("--//--"), None);
    assert_eq!(slugify("∑∏"), None);
}

#[test]
fn new_rejects_a_blank_statement() {
    let error = Problem::new("p", "  \n\t").unwrap_err();
    assert!(matches!(error, Error::EmptyProblem(slug) if slug == "p"));
}

#[test]
fn new_rejects_an_unusable_name() {
    let error = Problem::new("!!", "statement").unwrap_err();
    assert!(matches!(error, Error::UnnamedProblem(name) if name == "!!"));
}

#[test]
fn from_file_takes_the_slug_from_the_stem_and_trims_the_statement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Twin Primes.md");
    std::fs::write(&path, "\n# Twin primes\n\nAre there infinitely many?\n\n").unwrap();
    let problem = Problem::from_file(&path).unwrap();
    assert_eq!(problem.slug(), "twin-primes");
    assert_eq!(
        problem.statement(),
        "# Twin primes\n\nAre there infinitely many?"
    );
}

#[test]
fn from_file_reports_a_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let error = Problem::from_file(&dir.path().join("absent.md")).unwrap_err();
    assert!(matches!(error, Error::ProblemFile { .. }));
}
