//! Unit tests for the crate-wide error type.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Error;

#[test]
fn messages_are_lowercase_and_name_the_failing_piece() {
    let cases = [
        (
            Error::MissingConfig("SUPERMATH_MODEL"),
            "required setting `SUPERMATH_MODEL` is not set",
        ),
        (
            Error::InvalidConfig {
                name: "SUPERMATH_EMBEDDING_DIMS",
                value: "wide".into(),
            },
            "setting `SUPERMATH_EMBEDDING_DIMS` has an invalid value `wide`",
        ),
        (
            Error::DuplicateTool("calc_factor".into()),
            "tool `calc_factor` is registered twice",
        ),
        (
            Error::Runtime("no provider".into()),
            "openhuman runtime failed: no provider",
        ),
        (
            Error::Prompt {
                path: "problem.md".into(),
                reason: "not found".into(),
            },
            "cannot read the prompt `problem.md`: not found",
        ),
        (
            Error::Turn("deadline exceeded".into()),
            "agent turn failed: deadline exceeded",
        ),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn wrapped_errors_keep_the_inner_message() {
    let memory = Error::from(supermath_ledger::Error::MemoryNotReady("degraded".into()));
    assert_eq!(memory.to_string(), "memory is not ready: degraded");

    let toolkit = Error::from(supermath_mcp::Error::InvalidProject("!!".into()));
    assert_eq!(toolkit.to_string(), "invalid project name \"!!\"");

    let subagents = Error::from(supermath_subagents::Error::InvalidConfig {
        name: "SUPERMATH_READER_MODEL",
        value: " ".into(),
    });
    assert!(subagents.to_string().contains("SUPERMATH_READER_MODEL"));
}
