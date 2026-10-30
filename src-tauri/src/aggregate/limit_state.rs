//! Latest limit knowledge and its `LimitsView` (status ok / stale / unavailable).

use crate::contracts::limits::{
    LimitError, LimitReading, LimitReason, LimitStatus, LimitWindow, LimitsView,
};
use crate::contracts::EpochMs;
use crate::db::StoredLimit;
use crate::limits::LimitPoll;

use super::LIMIT_STALE_MS;

/// What the aggregate knows about limits: the last good reading and the
/// outcome of the latest failed poll, if any.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LimitState {
    /// Id of the source that produced `reading` (or failed last).
    pub source: Option<String>,
    /// Last good reading; kept across failures.
    pub reading: Option<LimitReading>,
    /// Error of the latest poll; cleared by the next good reading.
    pub last_error: Option<LimitError>,
}

impl LimitState {
    /// State restored from the newest stored snapshot.
    pub fn from_stored(stored: Option<StoredLimit>) -> Self {
        match stored {
            Some(s) => Self {
                source: Some(s.source),
                reading: Some(s.reading),
                last_error: None,
            },
            None => Self::default(),
        }
    }

    /// Folds one poll outcome in. Returns the reading if it was a good one
    /// (the caller stores it).
    pub fn apply(&mut self, poll: &LimitPoll) -> Option<&LimitReading> {
        self.source = Some(poll.source.to_owned());
        match &poll.result {
            Ok(reading) => {
                self.reading = Some(reading.clone());
                self.last_error = None;
                self.reading.as_ref()
            }
            Err(e) => {
                self.last_error = Some(e.clone());
                None
            }
        }
    }

    /// The view for the snapshot at time `now`.
    pub fn view(&self, now: EpochMs) -> LimitsView {
        let no_source = self
            .last_error
            .as_ref()
            .is_some_and(|e| e.reason == LimitReason::NoSource);
        let unavailable = |reason: LimitReason, detail: Option<String>| LimitsView {
            status: LimitStatus::Unavailable,
            reason: Some(reason),
            detail,
            source: self.source.clone(),
            five_hour: None,
            weekly: None,
            fetched_at: None,
        };

        let reading = match (&self.reading, no_source) {
            (Some(r), false) => r,
            (_, true) => {
                let e = self.last_error.as_ref();
                return unavailable(
                    LimitReason::NoSource,
                    e.map(|e| e.detail.clone()).filter(|d| !d.is_empty()),
                );
            }
            (None, false) => {
                return match &self.last_error {
                    Some(e) => unavailable(e.reason, Some(e.detail.clone())),
                    None => unavailable(LimitReason::Other, Some("no reading yet".into())),
                };
            }
        };

        let age = now.saturating_sub(reading.fetched_at);
        let (status, reason, detail) = if age <= LIMIT_STALE_MS {
            (LimitStatus::Ok, None, None)
        } else if let Some(e) = &self.last_error {
            (
                LimitStatus::Stale,
                Some(e.reason),
                Some(e.detail.clone()).filter(|d| !d.is_empty()),
            )
        } else {
            (
                LimitStatus::Stale,
                Some(LimitReason::Other),
                Some("last reading is old".into()),
            )
        };
        LimitsView {
            status,
            reason,
            detail,
            source: self.source.clone(),
            five_hour: reading.five_hour.clone(),
            weekly: reading.weekly.clone(),
            fetched_at: Some(reading.fetched_at),
        }
    }
}

/// Minutes until the 5-hour limit at the current pace, rounded down
/// (docs/ARCHITECTURE.md, "Limit ETA").
///
/// `window_tokens` are the headline tokens since the window started; the
/// percent per token is `utilization / window_tokens`. `None` without
/// tokens in the window or without burn; `Some(0)` at or above 100 %.
pub fn limit_eta_minutes(
    five_hour: &LimitWindow,
    now: EpochMs,
    tokens_per_minute: f64,
    window_tokens: u64,
) -> Option<u32> {
    if five_hour.resets_at.is_some_and(|r| r <= now) {
        // The window has ended; the reading says nothing about the next one.
        return None;
    }
    if five_hour.utilization >= 100.0 {
        return Some(0);
    }
    if window_tokens == 0 || tokens_per_minute <= 0.0 || five_hour.utilization <= 0.0 {
        return None;
    }
    let percent_per_token = five_hour.utilization / window_tokens as f64;
    let percent_per_minute = tokens_per_minute * percent_per_token;
    let minutes = (100.0 - five_hour.utilization) / percent_per_minute;
    minutes.is_finite().then(|| minutes.floor() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    fn reading(at: i64) -> LimitReading {
        LimitReading {
            five_hour: Some(LimitWindow {
                utilization: 40.0,
                resets_at: Some(at + 100 * MIN),
            }),
            weekly: None,
            fetched_at: at,
        }
    }

    fn err(reason: LimitReason, detail: &str) -> LimitError {
        LimitError {
            reason,
            detail: detail.into(),
        }
    }

    #[test]
    fn empty_state_is_unavailable() {
        let v = LimitState::default().view(1000);
        assert_eq!(v.status, LimitStatus::Unavailable);
        assert!(v.five_hour.is_none() && v.fetched_at.is_none());
    }

    #[test]
    fn no_source_error_is_unavailable_even_with_old_reading() {
        let mut s = LimitState::from_stored(Some(StoredLimit {
            source: "x".into(),
            reading: reading(0),
        }));
        s.apply(&LimitPoll {
            source: "none",
            result: Err(err(LimitReason::NoSource, "no source")),
        });
        let v = s.view(MIN);
        assert_eq!(v.status, LimitStatus::Unavailable);
        assert_eq!(v.reason, Some(LimitReason::NoSource));
        assert!(v.five_hour.is_none());
        assert_eq!(v.source.as_deref(), Some("none"));
    }

    #[test]
    fn fresh_reading_ok_old_reading_stale_with_last_error() {
        let mut s = LimitState::default();
        let r = reading(0);
        let poll = LimitPoll {
            source: "s",
            result: Ok(r),
        };
        assert!(s.apply(&poll).is_some());
        let v = s.view(15 * MIN);
        assert_eq!(v.status, LimitStatus::Ok);
        assert_eq!(v.reason, None);
        assert_eq!(v.five_hour.as_ref().unwrap().utilization, 40.0);

        s.apply(&LimitPoll {
            source: "s",
            result: Err(err(LimitReason::Network, "timeout")),
        });
        let v = s.view(15 * MIN + 1);
        assert_eq!(v.status, LimitStatus::Stale);
        assert_eq!(v.reason, Some(LimitReason::Network));
        assert_eq!(v.detail.as_deref(), Some("timeout"));
        // Values are still shown while stale.
        assert!(v.five_hour.is_some());
        assert_eq!(v.fetched_at, Some(0));
    }

    #[test]
    fn failure_before_any_reading_carries_the_reason() {
        let mut s = LimitState::default();
        s.apply(&LimitPoll {
            source: "s",
            result: Err(err(LimitReason::NotSignedIn, "expired")),
        });
        let v = s.view(0);
        assert_eq!(v.status, LimitStatus::Unavailable);
        assert_eq!(v.reason, Some(LimitReason::NotSignedIn));
    }

    #[test]
    fn eta_extrapolates_percent_per_token() {
        // 40 % used by 4000 tokens: 0.01 %/token. 100 tokens/min -> 1 %/min -> 60 min left.
        let w = LimitWindow {
            utilization: 40.0,
            resets_at: Some(10_000 * MIN),
        };
        assert_eq!(limit_eta_minutes(&w, 0, 100.0, 4000), Some(60));
        // Rounds down.
        assert_eq!(limit_eta_minutes(&w, 0, 99.0, 4000), Some(60));
        assert_eq!(limit_eta_minutes(&w, 0, 101.0, 4000), Some(59));
    }

    #[test]
    fn eta_edge_cases() {
        let mut w = LimitWindow {
            utilization: 40.0,
            resets_at: Some(10 * MIN),
        };
        assert_eq!(limit_eta_minutes(&w, 0, 0.0, 4000), None);
        assert_eq!(limit_eta_minutes(&w, 0, 10.0, 0), None);
        // Window already over.
        assert_eq!(limit_eta_minutes(&w, 10 * MIN, 10.0, 4000), None);
        w.utilization = 100.0;
        assert_eq!(limit_eta_minutes(&w, 0, 0.0, 0), Some(0));
        w.utilization = 140.0;
        assert_eq!(limit_eta_minutes(&w, 0, 10.0, 10), Some(0));
        w.utilization = 0.0;
        assert_eq!(limit_eta_minutes(&w, 0, 10.0, 10), None);
    }
}
