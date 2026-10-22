//! Pins the mapping from each failure to its envelope error code.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use supermath_tool::ErrorCode;

use super::Error;
use crate::path::Language;

fn io() -> std::io::Error {
    std::io::Error::other("disk")
}

#[test]
fn every_error_maps_to_its_documented_code() {
    let json_error = serde_json::from_str::<u8>("x").unwrap_err();
    let cases = [
        (Error::InvalidSlug("X".into()), ErrorCode::InvalidInput),
        (
            Error::InvalidPath {
                path: "a".into(),
                reason: "r",
            },
            ErrorCode::InvalidInput,
        ),
        (
            Error::ForbiddenImport {
                language: Language::Lean,
                module: "Std".into(),
                reason: "r",
            },
            ErrorCode::InvalidInput,
        ),
        (Error::GeneratedPath("a".into()), ErrorCode::InvalidInput),
        (Error::NotUtf8("a".into()), ErrorCode::InvalidInput),
        (Error::NotFound("a".into()), ErrorCode::NotFound),
        (
            Error::Conflict {
                path: "a".into(),
                expected: "x".into(),
                actual: "y".into(),
            },
            ErrorCode::Conflict,
        ),
        (
            Error::TooLarge { bytes: 2, limit: 1 },
            ErrorCode::BudgetExceeded,
        ),
        (Error::io("read", io()), ErrorCode::BackendUnavailable),
        (
            Error::LogCorrupt {
                line: 1,
                source: json_error,
            },
            ErrorCode::BackendUnavailable,
        ),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error}");
        assert_ne!(error.to_string(), "");
    }
}
