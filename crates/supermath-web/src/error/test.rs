//! Tests for the error-to-envelope mapping.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use supermath_tool::ErrorCode;

use super::Error;

#[test]
fn maps_every_variant_to_its_documented_code() {
    let cases = [
        (Error::MissingApiKey, ErrorCode::BackendUnavailable),
        (
            Error::Unavailable("x".into()),
            ErrorCode::BackendUnavailable,
        ),
        (Error::Decode("x".into()), ErrorCode::BackendUnavailable),
        (Error::Client("x".into()), ErrorCode::BackendUnavailable),
        (Error::TooManyRedirects, ErrorCode::BackendUnavailable),
        (Error::RedirectLoop, ErrorCode::BackendUnavailable),
        (Error::DeadEnd(404), ErrorCode::BackendUnavailable),
        (
            Error::InvalidConfig {
                variable: "V",
                reason: "r".into(),
            },
            ErrorCode::BackendUnavailable,
        ),
        (Error::Timeout, ErrorCode::Timeout),
        (Error::DailyCapSpent { limit: 1 }, ErrorCode::BudgetExceeded),
        (Error::InvalidUrl("x".into()), ErrorCode::InvalidInput),
        (Error::PrivateAddress("x".into()), ErrorCode::InvalidInput),
        (
            Error::UnsupportedContent("x".into()),
            ErrorCode::InvalidInput,
        ),
        (Error::TooLarge { limit: 1 }, ErrorCode::InvalidInput),
        (
            Error::ArxivPaper { arxiv_id: None },
            ErrorCode::InvalidInput,
        ),
        (Error::NotFound, ErrorCode::NotFound),
        (Error::Forbidden("x".into()), ErrorCode::Forbidden),
        (
            Error::RateLimited { retry_after_ms: 5 },
            ErrorCode::RateLimited,
        ),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error}");
        assert_eq!(error.clone().into_envelope().code(), Some(code));
    }
}

#[test]
fn a_rate_limit_envelope_carries_retry_after() {
    let envelope = Error::RateLimited {
        retry_after_ms: 1500,
    }
    .into_envelope();
    assert_eq!(envelope.error.and_then(|e| e.retry_after_ms), Some(1500));
}

#[test]
fn an_arxiv_refusal_suggests_lit_get_paper_when_it_names_a_paper() {
    let named = Error::ArxivPaper {
        arxiv_id: Some("1908.08483".into()),
    }
    .into_envelope();
    assert_eq!(named.next, [r#"lit_get_paper {"arxiv_id": "1908.08483"}"#]);
    let unnamed = Error::ArxivPaper { arxiv_id: None }.into_envelope();
    assert!(unnamed.next.is_empty());
}
