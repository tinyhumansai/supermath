//! A clock port, so rate limits and cache ages are testable without waiting.
//!
//! Production code uses [`SystemClock`]; tests use [`ManualClock`], whose
//! `sleep_ms` advances its own time instead of waiting.

use std::fmt::Debug;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

/// Time and sleeping, as the rate limiter and the cache need them.
#[async_trait]
pub trait Clock: Send + Sync + Debug {
    /// Milliseconds since the Unix epoch (or since the clock's own origin).
    fn now_ms(&self) -> u64;
    /// Waits `ms` milliseconds.
    async fn sleep_ms(&self, ms: u64);
}

/// The wall clock and a real `tokio` sleep.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

#[async_trait]
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
    }

    async fn sleep_ms(&self, ms: u64) {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
}

/// A clock that moves only when told to, and records every sleep.
///
/// ```
/// use supermath_literature::{Clock, ManualClock};
///
/// let clock = ManualClock::new(1_000);
/// clock.advance(500);
/// assert_eq!(clock.now_ms(), 1_500);
/// ```
#[derive(Debug, Default)]
pub struct ManualClock {
    now: AtomicU64,
    sleeps: Mutex<Vec<u64>>,
}

impl ManualClock {
    /// A clock reading `start_ms`.
    #[must_use]
    pub fn new(start_ms: u64) -> Self {
        Self {
            now: AtomicU64::new(start_ms),
            sleeps: Mutex::new(Vec::new()),
        }
    }

    /// Moves time forward by `ms`.
    pub fn advance(&self, ms: u64) {
        self.now.fetch_add(ms, Ordering::SeqCst);
    }

    /// Every duration passed to `sleep_ms`, in order.
    #[must_use]
    pub fn sleeps(&self) -> Vec<u64> {
        self.sleeps
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[async_trait]
impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }

    async fn sleep_ms(&self, ms: u64) {
        self.sleeps
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(ms);
        self.advance(ms);
    }
}

#[cfg(test)]
mod test;
