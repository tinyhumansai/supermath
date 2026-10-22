//! The mapping from errors to tool error codes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use supermath_tool::ErrorCode;

use super::Error;

#[test]
fn every_error_maps_to_the_documented_code() {
    let cases = [
        (Error::MissingConfig("K"), ErrorCode::BackendUnavailable),
        (Error::InvalidProject("!".into()), ErrorCode::InvalidInput),
        (
            Error::MemoryTransport {
                operation: "recall",
                detail: String::new(),
            },
            ErrorCode::BackendUnavailable,
        ),
        (
            Error::MemoryRefused {
                operation: "recall",
                status: 403,
                detail: String::new(),
            },
            ErrorCode::BackendUnavailable,
        ),
        (
            Error::MemoryNotReady(String::new()),
            ErrorCode::BackendUnavailable,
        ),
        (
            Error::MemoryNotIndexed {
                completed: String::new(),
            },
            ErrorCode::BackendUnavailable,
        ),
        (
            Error::InvalidArguments(String::new()),
            ErrorCode::InvalidInput,
        ),
        (
            Error::StatusFromCaller("formalised".into()),
            ErrorCode::Forbidden,
        ),
        (Error::OriginFromCaller, ErrorCode::Forbidden),
        (Error::UnrecordedCall("c".into()), ErrorCode::Forbidden),
        (
            Error::CallNotUsable {
                call_id: "c".into(),
                reason: "r",
            },
            ErrorCode::Forbidden,
        ),
        (Error::MissingFrame, ErrorCode::InvalidInput),
        (
            Error::StaleDigest {
                path: "p".into(),
                given: "a".into(),
                current: "b".into(),
            },
            ErrorCode::Conflict,
        ),
        (Error::ClaimNotFound("c".into()), ErrorCode::NotFound),
        (
            Error::LinkCycle {
                from: "a".into(),
                to: "b".into(),
            },
            ErrorCode::InvalidInput,
        ),
        (
            Error::EvidenceNotFound {
                path: "p".into(),
                declaration: "d".into(),
            },
            ErrorCode::NotFound,
        ),
        (
            Error::EvidenceUnavailable(String::new()),
            ErrorCode::BackendUnavailable,
        ),
        (Error::EvidenceTimeout, ErrorCode::Timeout),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error}");
        assert_ne!(error.to_string().len(), 0);
    }
}
