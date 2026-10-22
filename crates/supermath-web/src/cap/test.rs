//! Tests for the daily cap.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::DailyCap;
use crate::error::Error;

#[test]
fn refuses_past_the_limit_and_resets_the_next_day() {
    let cap = DailyCap::new(2);
    assert!(cap.acquire("p", 100).is_ok());
    assert!(cap.acquire("p", 200).is_ok());
    assert_eq!(
        cap.acquire("p", 300),
        Err(Error::DailyCapSpent { limit: 2 })
    );
    assert!(cap.acquire("p", 86_400 + 1).is_ok());
}

#[test]
fn counts_each_project_separately() {
    let cap = DailyCap::new(1);
    assert!(cap.acquire("a", 0).is_ok());
    assert!(cap.acquire("b", 0).is_ok());
    assert!(cap.acquire("a", 0).is_err());
}
