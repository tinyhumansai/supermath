//! Tests for the rate limiter, always on a fake clock.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use super::RateLimiter;
use crate::ManualClock;

fn limiter(interval: u64) -> (RateLimiter, Arc<ManualClock>) {
    let clock = Arc::new(ManualClock::new(1_000));
    (RateLimiter::new(interval, clock.clone()), clock)
}

#[test]
fn the_first_request_goes_at_once_and_the_rest_queue() {
    let (limiter, _) = limiter(3_000);
    assert_eq!(limiter.reserve(), 0);
    assert_eq!(limiter.reserve(), 3_000);
    assert_eq!(limiter.reserve(), 6_000);
}

#[test]
fn time_passing_shortens_the_wait() {
    let (limiter, clock) = limiter(3_000);
    assert_eq!(limiter.reserve(), 0);
    clock.advance(1_000);
    assert_eq!(limiter.reserve(), 2_000);
}

#[test]
fn an_idle_limiter_forgets_the_past() {
    let (limiter, clock) = limiter(3_000);
    assert_eq!(limiter.reserve(), 0);
    clock.advance(60_000);
    assert_eq!(limiter.reserve(), 0);
}

#[test]
fn a_zero_interval_never_waits() {
    let (limiter, _) = limiter(0);
    assert_eq!(limiter.reserve(), 0);
    assert_eq!(limiter.reserve(), 0);
}

#[tokio::test]
async fn acquire_sleeps_only_for_the_queued_requests() {
    let (limiter, clock) = limiter(3_000);
    limiter.acquire().await;
    limiter.acquire().await;
    limiter.acquire().await;
    assert_eq!(clock.sleeps(), vec![3_000, 3_000]);
}
