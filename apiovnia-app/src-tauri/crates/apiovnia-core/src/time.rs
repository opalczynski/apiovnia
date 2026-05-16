//! Wall-clock helpers.
//!
//! Apiovnia stores timestamps as Unix epoch milliseconds (i64). This module
//! is the single source for that conversion and gives us a seam to inject a
//! deterministic clock in tests via `Clock::Fixed`.

use chrono::{DateTime, Utc};

/// Source of "now". Production code uses [`Clock::system`]; tests use
/// [`Clock::fixed`].
#[derive(Debug, Clone)]
pub enum Clock {
    System,
    Fixed(i64),
}

impl Clock {
    #[must_use]
    pub fn system() -> Self {
        Self::System
    }

    #[must_use]
    pub fn fixed(epoch_ms: i64) -> Self {
        Self::Fixed(epoch_ms)
    }

    #[must_use]
    pub fn now_millis(&self) -> i64 {
        match self {
            Self::System => Utc::now().timestamp_millis(),
            Self::Fixed(v) => *v,
        }
    }
}

#[must_use]
pub fn epoch_millis_now() -> i64 {
    Utc::now().timestamp_millis()
}

#[must_use]
pub fn millis_to_datetime(ms: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_else(|| {
        // Out-of-range values are clamped — should never happen with sqlite-stored
        // timestamps but we don't want a panic in the IPC layer.
        DateTime::<Utc>::from_timestamp_millis(0).expect("epoch 0 is always valid")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_is_deterministic() {
        let c = Clock::fixed(1_700_000_000_000);
        assert_eq!(c.now_millis(), 1_700_000_000_000);
        assert_eq!(c.now_millis(), 1_700_000_000_000);
    }

    #[test]
    fn round_trip_millis_to_datetime() {
        let dt = millis_to_datetime(1_700_000_000_000);
        assert_eq!(dt.timestamp_millis(), 1_700_000_000_000);
    }
}
