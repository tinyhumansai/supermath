//! Tests for the error-to-envelope mapping.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use supermath_tool::{Envelope, ErrorCode};

use super::Error;

fn unavailable() -> Error {
    Error::Unavailable {
        source_name: "arxiv".into(),
        reason: "connection refused".into(),
    }
}

#[test]
fn every_variant_maps_to_one_stable_code() {
    let cases = [
        (Error::InvalidInput("x".into()), ErrorCode::InvalidInput),
        (Error::NotFound("x".into()), ErrorCode::NotFound),
        (unavailable(), ErrorCode::BackendUnavailable),
        (
            Error::Timeout {
                source_name: "oeis".into(),
            },
            ErrorCode::Timeout,
        ),
        (
            Error::RateLimited {
                source_name: "arxiv".into(),
                retry_after_ms: 1,
            },
            ErrorCode::RateLimited,
        ),
        (
            Error::Upstream {
                source_name: "oeis".into(),
                status: 500,
            },
            ErrorCode::BackendUnavailable,
        ),
        (
            Error::Decode {
                what: "feed".into(),
                reason: "bad".into(),
            },
            ErrorCode::BackendUnavailable,
        ),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code);
        assert_eq!(Envelope::from(error).code(), Some(code));
    }
}

#[test]
fn a_rate_limit_keeps_its_retry_after() {
    let envelope = Envelope::from(Error::RateLimited {
        source_name: "arxiv".into(),
        retry_after_ms: 4200,
    });
    assert_eq!(
        envelope.error.and_then(|info| info.retry_after_ms),
        Some(4200)
    );
}

#[test]
fn other_errors_carry_no_retry_after() {
    let envelope = Envelope::from(unavailable());
    assert_eq!(envelope.error.and_then(|info| info.retry_after_ms), None);
}

#[test]
fn messages_are_lowercase_without_trailing_punctuation() {
    let message = unavailable().to_string();
    assert_eq!(message, "arxiv is unavailable: connection refused");
}
