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
            Error::InvalidProject("!!".into()),
            "project name `!!` has no usable characters",
        ),
        (
            Error::MemoryNotReady("embeddings: no key".into()),
            "memory is not ready: embeddings: no key",
        ),
        (
            Error::MemoryNotIndexed {
                completed: "stored".into(),
            },
            "memory stored the note but did not index it (completed: stored)",
        ),
        (
            Error::MemoryRefused {
                operation: "recall",
                status: 403,
                detail: "POLICY_DENIED".into(),
            },
            "memory refused `recall` with status 403: POLICY_DENIED",
        ),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}
