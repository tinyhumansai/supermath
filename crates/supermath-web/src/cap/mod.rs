//! The daily cap on `web_*` calls per project.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::clock::DAY_SECS;
use crate::error::{Error, Result};

/// Counts calls per project per UTC day and refuses past the limit.
///
/// The count lives in memory: a restart resets it, which is the cheap side
/// to err on for a guard against runaway loops rather than a billing meter.
#[derive(Debug)]
pub(crate) struct DailyCap {
    limit: u32,
    counts: Mutex<HashMap<String, (u64, u32)>>,
}

impl DailyCap {
    pub(crate) fn new(limit: u32) -> Self {
        Self {
            limit,
            counts: Mutex::new(HashMap::new()),
        }
    }

    /// Records one call for `project` at `now_secs`.
    ///
    /// # Errors
    ///
    /// [`Error::DailyCapSpent`] when the project has already used its calls
    /// for the day.
    pub(crate) fn acquire(&self, project: &str, now_secs: u64) -> Result<()> {
        let day = now_secs / DAY_SECS;
        let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = counts.entry(project.to_owned()).or_insert((day, 0));
        if entry.0 != day {
            *entry = (day, 0);
        }
        if entry.1 >= self.limit {
            return Err(Error::DailyCapSpent { limit: self.limit });
        }
        entry.1 += 1;
        Ok(())
    }
}

#[cfg(test)]
mod test;
