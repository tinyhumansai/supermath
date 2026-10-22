//! The politeness limiter: at most one request per interval, process-wide.
//!
//! arXiv asks for one request every three seconds. A [`RateLimiter`] hands out
//! consecutive time slots, so concurrent callers queue behind each other
//! instead of racing. It reads time from a [`Clock`], which is how its tests
//! run on a fake clock and never wait.

use std::sync::{Arc, Mutex, PoisonError};

use crate::Clock;

/// Spaces requests at least `interval_ms` apart.
///
/// ```
/// use std::sync::Arc;
/// use supermath_literature::{ManualClock, RateLimiter};
///
/// let limiter = RateLimiter::new(3_000, Arc::new(ManualClock::new(0)));
/// assert_eq!(limiter.reserve(), 0); // the first request goes at once
/// assert_eq!(limiter.reserve(), 3_000); // the second waits its turn
/// ```
#[derive(Debug)]
pub struct RateLimiter {
    interval_ms: u64,
    next_slot_ms: Mutex<Option<u64>>,
    clock: Arc<dyn Clock>,
}

impl RateLimiter {
    /// A limiter allowing one request per `interval_ms`; zero disables it.
    #[must_use]
    pub fn new(interval_ms: u64, clock: Arc<dyn Clock>) -> Self {
        Self {
            interval_ms,
            next_slot_ms: Mutex::new(None),
            clock,
        }
    }

    /// Books the next free slot and returns how long to wait for it, in
    /// milliseconds. The booking happens now; the waiting does not.
    #[must_use]
    pub fn reserve(&self) -> u64 {
        let now = self.clock.now_ms();
        let mut next = self
            .next_slot_ms
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let slot = next.map_or(now, |n| n.max(now));
        *next = Some(slot.saturating_add(self.interval_ms));
        slot - now
    }

    /// Waits for the next free slot.
    pub async fn acquire(&self) {
        let wait = self.reserve();
        if wait > 0 {
            self.clock.sleep_ms(wait).await;
        }
    }
}

#[cfg(test)]
mod test;
