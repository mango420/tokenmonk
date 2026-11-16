//! The single limit notification: 10 % left, once per window and reset cycle
//! (docs/DESIGN.md, "Product line"). Pure logic, no I/O: the caller feeds
//! the windows of each new reading to [`LimitNotifier::observe`] and shows
//! what it returns. A window without a reset time never notifies. The notified cycles ([`Cycles`]) are stored by the caller under
//! `app_state` key [`STATE_KEY`], so a restart does not repeat a notice.

use chrono::{DateTime, TimeZone};
use serde::{Deserialize, Serialize};

use crate::contracts::limits::{LimitReading, LimitWindow};
use crate::contracts::EpochMs;

/// Notify when this share or less is left, in percent.
pub const NOTIFY_AT_LEFT: f64 = 10.0;

/// Which limit window a notification is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    FiveHour,
    Weekly,
}

/// What to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitNotice {
    pub window: WindowKind,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Default, Clone, Copy)]
struct Slot {
    /// A notification went out for the current cycle.
    fired: bool,
    /// Reset time of the cycle that fired.
    cycle: Option<EpochMs>,
}

/// Remembers which cycles already notified.
#[derive(Debug, Default)]
pub struct LimitNotifier {
    five_hour: Slot,
    seven_day: Slot,
}

/// `app_state` key of the stored [`Cycles`].
pub const STATE_KEY: &str = "limit_notice_cycles";

/// The cycles that already notified, as stored: the `resetsAt` of each
/// window's notified cycle, `null` if none (docs/ARCHITECTURE.md, "Storage").
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cycles {
    pub five_hour: Option<EpochMs>,
    pub weekly: Option<EpochMs>,
}

impl LimitNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// A notifier that knows which cycles were already notified.
    pub fn with_cycles(cycles: Cycles) -> Self {
        let slot = |cycle: Option<EpochMs>| Slot {
            fired: cycle.is_some(),
            cycle,
        };
        Self {
            five_hour: slot(cycles.five_hour),
            seven_day: slot(cycles.weekly),
        }
    }

    /// The notified cycles to store.
    pub fn cycles(&self) -> Cycles {
        let cycle = |s: &Slot| s.cycle.filter(|_| s.fired);
        Cycles {
            five_hour: cycle(&self.five_hour),
            weekly: cycle(&self.seven_day),
        }
    }

    /// Like [`Self::check`] for two windows, formatted in the local zone.
    pub fn observe(
        &mut self,
        five_hour: Option<&LimitWindow>,
        weekly: Option<&LimitWindow>,
        now: EpochMs,
    ) -> Vec<LimitNotice> {
        self.observe_in(five_hour, weekly, now, &chrono::Local)
    }

    /// [`Self::observe`] with an explicit zone.
    pub fn observe_in<Tz>(
        &mut self,
        five_hour: Option<&LimitWindow>,
        weekly: Option<&LimitWindow>,
        now: EpochMs,
        tz: &Tz,
    ) -> Vec<LimitNotice>
    where
        Tz: TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        let mut out = Vec::new();
        for (kind, window, slot) in [
            (WindowKind::FiveHour, five_hour, &mut self.five_hour),
            (WindowKind::Weekly, weekly, &mut self.seven_day),
        ] {
            if let Some(n) = check_window(kind, window, slot, now, tz) {
                out.push(n);
            }
        }
        out
    }

    /// Returns the notices due for `reading`, at most one per window.
    /// `tz` formats the reset time (the user's local zone).
    pub fn check<Tz>(&mut self, reading: &LimitReading, now: EpochMs, tz: &Tz) -> Vec<LimitNotice>
    where
        Tz: TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        self.observe_in(reading.five_hour.as_ref(), reading.weekly.as_ref(), now, tz)
    }
}

fn check_window<Tz>(
    kind: WindowKind,
    window: Option<&LimitWindow>,
    slot: &mut Slot,
    now: EpochMs,
    tz: &Tz,
) -> Option<LimitNotice>
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display,
{
    // A dropped window means it reset: the next cycle starts clean.
    let Some(w) = window.filter(|w| w.utilization.is_finite()) else {
        *slot = Slot::default();
        return None;
    };
    let left = (100.0 - w.utilization).clamp(0.0, 100.0);
    // No reset time, no cycle to tell apart: stay silent, keep the slot.
    let resets_at = w.resets_at?;
    // A different `resetsAt` is a new cycle (compared exactly).
    if slot.cycle != Some(resets_at) {
        *slot = Slot::default();
    }
    if left > NOTIFY_AT_LEFT || slot.fired {
        return None;
    }
    *slot = Slot {
        fired: true,
        cycle: Some(resets_at),
    };
    Some(LimitNotice {
        window: kind,
        title: title(kind).into(),
        text: body(left, Some(resets_at), now, tz),
    })
}

fn title(kind: WindowKind) -> &'static str {
    match kind {
        WindowKind::FiveHour => "Limits: 5-hour",
        WindowKind::Weekly => "Limits: 7-day",
    }
}

/// "10% left. Resets 16:40." Another day adds the weekday ("Resets Thu 16:40").
fn body<Tz: TimeZone>(left: f64, resets_at: Option<EpochMs>, now: EpochMs, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let left = left.round() as i64;
    let Some(at) = resets_at.and_then(DateTime::from_timestamp_millis) else {
        return format!("{left}% left.");
    };
    let at = at.with_timezone(tz);
    let today = DateTime::from_timestamp_millis(now).map(|n| n.with_timezone(tz).date_naive());
    if today == Some(at.date_naive()) {
        format!("{left}% left. Resets {}.", at.format("%H:%M"))
    } else {
        format!("{left}% left. Resets {}.", at.format("%a %H:%M"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    const H: i64 = 3_600_000;
    // 2026-01-05 12:00 UTC, a Monday.
    const NOON: i64 = 1_767_614_400_000;

    fn utc() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn win(used: f64, resets: Option<i64>) -> Option<LimitWindow> {
        Some(LimitWindow {
            utilization: used,
            resets_at: resets,
        })
    }

    fn reading(five: Option<LimitWindow>, week: Option<LimitWindow>) -> LimitReading {
        LimitReading {
            five_hour: five,
            weekly: week,
            fetched_at: NOON,
        }
    }

    #[test]
    fn silent_above_ten_percent_left() {
        let mut n = LimitNotifier::default();
        let r = reading(win(89.0, Some(NOON + H)), win(10.0, Some(NOON + 90 * H)));
        assert!(n.check(&r, NOON, &utc()).is_empty());
    }

    #[test]
    fn fires_once_per_cycle_with_dry_text() {
        let mut n = LimitNotifier::default();
        let r = reading(win(90.0, Some(NOON + 4 * H + 40 * 60_000)), None);
        let got = n.check(&r, NOON, &utc());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].window, WindowKind::FiveHour);
        assert_eq!(got[0].text, "10% left. Resets 16:40.");
        let worse = reading(win(97.0, Some(NOON + 4 * H + 40 * 60_000)), None);
        assert!(n.check(&worse, NOON, &utc()).is_empty());
    }

    #[test]
    fn fires_again_in_the_next_cycle() {
        let mut n = LimitNotifier::default();
        let a = reading(win(95.0, Some(NOON + H)), None);
        assert_eq!(n.check(&a, NOON, &utc()).len(), 1);
        let next = reading(win(92.0, Some(NOON + 6 * H)), None);
        assert_eq!(n.check(&next, NOON + 2 * H, &utc()).len(), 1);
    }

    #[test]
    fn rearms_after_the_window_drops() {
        let mut n = LimitNotifier::default();
        let a = reading(win(95.0, Some(NOON + H)), None);
        assert_eq!(n.check(&a, NOON, &utc()).len(), 1);
        assert!(n
            .check(&reading(None, None), NOON + 2 * H, &utc())
            .is_empty());
        assert_eq!(n.check(&a, NOON + 2 * H, &utc()).len(), 1);
    }

    #[test]
    fn windows_are_independent_and_weekday_shows_on_other_days() {
        let mut n = LimitNotifier::default();
        let r = reading(win(91.0, Some(NOON + H)), win(96.0, Some(NOON + 72 * H)));
        let got = n.check(&r, NOON, &utc());
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].text, "9% left. Resets 13:00.");
        assert_eq!(got[1].window, WindowKind::Weekly);
        assert_eq!(got[1].text, "4% left. Resets Thu 12:00.");
    }

    #[test]
    fn window_without_reset_time_sends_nothing() {
        let mut n = LimitNotifier::default();
        let hot = reading(win(95.0, None), None);
        assert!(n.check(&hot, NOON, &utc()).is_empty());
        assert_eq!(n.cycles(), Cycles::default());
    }
}
