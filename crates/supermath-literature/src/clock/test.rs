//! Tests for the system and manual clocks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Clock, ManualClock, SystemClock};

#[test]
fn the_system_clock_reads_a_plausible_epoch_time() {
    // After 2020-01-01.
    assert!(SystemClock.now_ms() > 1_577_836_800_000);
}

#[tokio::test]
async fn the_system_clock_really_sleeps() {
    let before = SystemClock.now_ms();
    SystemClock.sleep_ms(5).await;
    assert!(SystemClock.now_ms() >= before + 4);
}

#[tokio::test]
async fn a_manual_sleep_advances_time_and_is_recorded() {
    let clock = ManualClock::new(10);
    clock.sleep_ms(30).await;
    clock.sleep_ms(5).await;
    assert_eq!(clock.now_ms(), 45);
    assert_eq!(clock.sleeps(), vec![30, 5]);
}
