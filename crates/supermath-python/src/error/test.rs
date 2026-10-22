//! Tests for the error-to-envelope mapping.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use supermath_tool::{Envelope, ErrorCode};

use super::Error;

#[test]
fn each_error_maps_to_its_code() {
    let cases = [
        (Error::InvalidInput("x".into()), ErrorCode::InvalidInput),
        (Error::NotFound("x".into()), ErrorCode::NotFound),
        (Error::BudgetExceeded("x".into()), ErrorCode::BudgetExceeded),
        (
            Error::Unavailable("x".into()),
            ErrorCode::BackendUnavailable,
        ),
        (
            Error::Upstream {
                status: 500,
                message: "x".into(),
            },
            ErrorCode::BackendUnavailable,
        ),
        (Error::Decode("x".into()), ErrorCode::BackendUnavailable),
        (
            Error::Files(supermath_files::Error::NotFound("p".into())),
            ErrorCode::NotFound,
        ),
    ];
    for (error, code) in cases {
        assert_eq!(error.code(), code, "{error}");
        let envelope = Envelope::from(error);
        assert_eq!(envelope.code(), Some(code));
        assert!(envelope.is_well_formed());
    }
}

#[test]
fn messages_name_the_cause() {
    let error = Error::Upstream {
        status: 502,
        message: "bad gateway".into(),
    };
    assert_eq!(
        error.to_string(),
        "the python service answered with status 502: bad gateway"
    );
}
