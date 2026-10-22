//! Unit tests for the error messages.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Error;
use crate::contract::Violation;

#[test]
fn messages_are_lowercase_and_unpunctuated() {
    let errors = [
        Error::InvalidProjectSlug("Bad Slug".into()),
        Error::EmptyProjectScope,
        Error::DuplicateTool("calc_gcd_lcm".into()),
        Error::UnknownTool("nope".into()),
    ];
    for error in errors {
        let text = error.to_string();
        assert!(
            text.chars().next().is_some_and(char::is_lowercase),
            "{text}"
        );
        assert!(!text.ends_with('.'), "{text}");
    }
}

#[test]
fn contract_error_lists_every_violation() {
    let error = Error::Contract {
        tool: "x".into(),
        violations: vec![Violation::NoFixtures, Violation::EmptyDescription],
    };
    let text = error.to_string();
    assert!(text.contains("no fixtures"), "{text}");
    assert!(text.contains("; "), "{text}");
}
