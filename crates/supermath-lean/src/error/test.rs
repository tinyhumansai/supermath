//! Unit tests for the error-to-envelope mapping.

#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use supermath_tool::{Envelope, ErrorCode};

use super::Error;

#[test]
fn each_variant_maps_to_its_code() {
    let cases = [
        (Error::InvalidInput("x".into()), ErrorCode::InvalidInput),
        (Error::NotFound("x".into()), ErrorCode::NotFound),
        (Error::Conflict("x".into()), ErrorCode::Conflict),
        (Error::BudgetExceeded("x".into()), ErrorCode::BudgetExceeded),
        (
            Error::unavailable("lean", "down"),
            ErrorCode::BackendUnavailable,
        ),
        (Error::Timeout("slow".into()), ErrorCode::Timeout),
        (
            Error::RateLimited {
                service: "loogle".into(),
                retry_after_ms: 5,
            },
            ErrorCode::RateLimited,
        ),
        (
            Error::Files(supermath_files::Error::NotFound("p".into())),
            ErrorCode::NotFound,
        ),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error}");
        assert_eq!(Envelope::from(&error).code(), Some(code));
    }
}

#[test]
fn a_rate_limit_keeps_its_retry_delay() {
    let envelope = Envelope::from(Error::RateLimited {
        service: "loogle".into(),
        retry_after_ms: 7_000,
    });
    assert_eq!(envelope.error.unwrap().retry_after_ms, Some(7_000));
}

#[test]
fn messages_are_lowercase_without_trailing_punctuation() {
    let text = Error::unavailable("lean", "connection failed").to_string();
    assert_eq!(text, "lean is unavailable: connection failed");
}
