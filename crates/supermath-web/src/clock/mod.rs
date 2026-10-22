//! Wall-clock time as a port, so the cache, the daily cap, and `retrieved_at`
//! stay testable without sleeping.

use std::time::{SystemTime, UNIX_EPOCH};

/// A source of the current time, in whole seconds since the Unix epoch.
pub trait Clock: Send + Sync + std::fmt::Debug {
    /// Seconds since 1970-01-01T00:00:00Z.
    fn now_secs(&self) -> u64;
}

/// The system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
}

/// Seconds in a day.
pub(crate) const DAY_SECS: u64 = 86_400;

/// The civil date of a day count since the epoch (Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// Formats `secs` since the epoch as `2026-10-22T14:03:00Z`.
#[must_use]
pub(crate) fn format_rfc3339(secs: u64) -> String {
    let days = i64::try_from(secs / DAY_SECS).unwrap_or(i64::MAX / 2);
    let (year, month, day) = civil_from_days(days);
    let rem = secs % DAY_SECS;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Whether `text` is a real calendar date written `YYYY-MM-DD`.
#[must_use]
pub(crate) fn is_valid_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let longest = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=longest).contains(&day)
}

#[cfg(test)]
mod test;
