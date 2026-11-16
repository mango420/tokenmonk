//! TKM-304: one notification at 10 percent left, once per window and reset
//! cycle. The old 90 percent and 100 percent warnings are gone.
//!
//! Utilization is "used percent", so 10 percent left is 90.0 used. The
//! assumed library API is isolated in `api` so it can be adapted in one
//! place once TKM-304 has landed.

mod common;

use common::utc_ms;

/// Assumed API of TKM-304 (names to verify after the coder PR is merged).
mod api {
    use tokenmonk_lib::contracts::limits::LimitWindow;
    use tokenmonk_lib::limits::notice::{LimitNotifier, WindowKind};

    #[derive(Debug, Clone, PartialEq)]
    pub struct Notice {
        pub five_hour: bool,
        pub text: String,
    }

    pub struct Rig(pub LimitNotifier);

    impl Rig {
        pub fn new() -> Self {
            Self(LimitNotifier::new())
        }

        /// Feeds one limit reading; returns the notifications that fire.
        pub fn feed(
            &mut self,
            five: Option<(f64, i64)>,
            week: Option<(f64, i64)>,
            now: i64,
        ) -> Vec<Notice> {
            let w = |x: Option<(f64, i64)>| {
                x.map(|(u, r)| LimitWindow {
                    utilization: u,
                    resets_at: Some(r),
                })
            };
            self.0
                .observe(w(five).as_ref(), w(week).as_ref(), now)
                .into_iter()
                .map(|n| Notice {
                    five_hour: n.window == WindowKind::FiveHour,
                    text: n.text,
                })
                .collect()
        }
    }
}

use api::Rig;
use tokenmonk_lib::limits::notice::{Cycles, LimitNotifier};

fn t(h: u32, m: u32) -> i64 {
    utc_ms(2026, 10, 28, h, m, 0)
}
fn reset1() -> i64 {
    t(12, 0)
}
fn reset2() -> i64 {
    t(17, 0)
}
fn week_reset() -> i64 {
    utc_ms(2026, 10, 30, 18, 0, 0)
}

#[test]
fn nothing_above_ten_percent_left() {
    let mut r = Rig::new();
    for u in [0.0, 50.0, 80.0, 89.9] {
        assert!(
            r.feed(Some((u, reset1())), None, t(10, 0)).is_empty(),
            "{u}"
        );
    }
}

#[test]
fn fires_once_at_ten_percent_left_and_stays_quiet_until_the_window_resets() {
    let mut r = Rig::new();
    assert!(r.feed(Some((85.0, reset1())), None, t(10, 0)).is_empty());

    let fired = r.feed(Some((90.0, reset1())), None, t(10, 20));
    assert_eq!(fired.len(), 1);
    assert!(fired[0].five_hour);
    assert!(fired[0].text.contains("10%"), "{}", fired[0].text);

    // Same cycle: 95, 99, 100 and repeats of 90 add nothing (no 90/100 warnings).
    for u in [90.0, 95.0, 99.0, 100.0, 100.0] {
        assert!(
            r.feed(Some((u, reset1())), None, t(10, 40)).is_empty(),
            "{u}"
        );
    }
}

#[test]
fn re_arms_after_the_reset_and_fires_again_in_the_next_cycle() {
    let mut r = Rig::new();
    assert_eq!(r.feed(Some((92.0, reset1())), None, t(11, 0)).len(), 1);

    // New window: low usage, new reset time.
    assert!(r.feed(Some((5.0, reset2())), None, t(12, 5)).is_empty());
    assert!(r.feed(Some((60.0, reset2())), None, t(14, 0)).is_empty());

    assert_eq!(r.feed(Some((91.0, reset2())), None, t(15, 0)).len(), 1);
    assert!(r.feed(Some((97.0, reset2())), None, t(15, 30)).is_empty());
}

#[test]
fn an_already_low_first_reading_of_a_new_cycle_fires_once() {
    let mut r = Rig::new();
    assert_eq!(r.feed(Some((95.0, reset1())), None, t(11, 0)).len(), 1);
    assert_eq!(r.feed(Some((93.0, reset2())), None, t(12, 5)).len(), 1);
}

#[test]
fn both_windows_are_tracked_independently() {
    let mut r = Rig::new();

    let first = r.feed(Some((90.0, reset1())), Some((40.0, week_reset())), t(10, 0));
    assert_eq!(first.len(), 1);
    assert!(first[0].five_hour);

    let second = r.feed(
        Some((95.0, reset1())),
        Some((91.0, week_reset())),
        t(10, 30),
    );
    assert_eq!(second.len(), 1);
    assert!(!second[0].five_hour, "weekly fires on its own");

    assert!(r
        .feed(Some((99.0, reset1())), Some((99.0, week_reset())), t(11, 0))
        .is_empty());
}

#[test]
fn both_windows_crossing_in_the_same_reading_give_one_notice_each() {
    let mut r = Rig::new();
    let fired = r.feed(Some((90.0, reset1())), Some((90.0, week_reset())), t(10, 0));
    assert_eq!(fired.len(), 2);
    assert_ne!(fired[0].five_hour, fired[1].five_hour);
}

#[test]
fn missing_windows_never_notify() {
    let mut r = Rig::new();
    assert!(r.feed(None, None, t(10, 0)).is_empty());
}

#[test]
fn a_restart_inside_the_same_cycle_does_not_repeat_the_notice() {
    let mut r = Rig::new();
    assert_eq!(r.feed(Some((92.0, reset1())), None, t(11, 0)).len(), 1);

    // The caller stores the cycles under app_state `limit_notice_cycles`
    // and hands them back to a fresh notifier after the restart.
    let stored = serde_json::to_value(r.0.cycles()).unwrap();
    assert_eq!(stored["fiveHour"], reset1());
    assert!(stored["weekly"].is_null());
    let restored: Cycles = serde_json::from_value(stored).unwrap();
    let mut after = Rig(LimitNotifier::with_cycles(restored));

    assert!(after
        .feed(Some((95.0, reset1())), None, t(11, 30))
        .is_empty());
    // The next cycle still notifies.
    assert_eq!(after.feed(Some((93.0, reset2())), None, t(12, 5)).len(), 1);
}
