//! Messages of the crate-wide error: lowercase, no trailing punctuation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Error;

#[test]
fn messages_are_lowercase_and_unpunctuated() {
    let errors = [
        Error::InvalidProject("***".into()),
        Error::Usage("usage: supermath-mcp serve [project]".into()),
        Error::Transport("closed".into()),
        Error::EnvFile {
            path: "a.env".into(),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        },
        Error::Tool(supermath_tool::Error::DuplicateTool("calc_factor".into())),
    ];
    for error in errors {
        let message = error.to_string();
        assert!(!message.ends_with('.'), "{message}");
        assert!(
            !message.chars().next().is_some_and(char::is_uppercase),
            "{message}"
        );
    }
}

#[test]
fn a_domain_error_keeps_its_own_message() {
    let error = Error::from(supermath_tool::Error::UnknownTool("nope".into()));
    assert!(error.to_string().contains("nope"));
}
