//! Tests for date formatting and validation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Clock, SystemClock, format_rfc3339, is_valid_date};

#[test]
fn formats_the_epoch_and_a_known_instant() {
    assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_rfc3339(1_792_677_780), "2026-10-22T14:03:00Z");
    assert_eq!(format_rfc3339(951_782_400), "2000-02-29T00:00:00Z");
}

#[test]
fn the_system_clock_is_after_2025() {
    assert!(SystemClock.now_secs() > 1_735_689_600);
}

#[test]
fn validates_calendar_dates() {
    for good in ["2019-01-01", "2024-02-29", "2000-02-29", "1999-12-31"] {
        assert!(is_valid_date(good), "{good}");
    }
    for bad in [
        "2019-1-1",
        "2019-13-01",
        "2019-00-10",
        "2023-02-29",
        "1900-02-29",
        "2019-04-31",
        "2019-01-32",
        "2019/01/01",
        "20190101",
        "",
        "abcd-ef-gh",
        "2019-01-00",
    ] {
        assert!(!is_valid_date(bad), "{bad}");
    }
}
