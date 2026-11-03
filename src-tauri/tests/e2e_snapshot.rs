//! End to end: fixture logs -> incremental ingest -> dashboard snapshot ->
//! halo value per mode. Expected numbers are computed by hand from the
//! message table in `ingest_fixtures.rs` (all times UTC, the test zone).
//!
//! At 2026-10-28 11:40 the fixture day (28th) holds:
//!   M3 08:15  4 /  700 / 1000 / 250000
//!   M4 11:30  4 /  300 /    0 / 260000
//! so today is 8 / 1000 / 1000 / 510000 with a headline of 2008, and the
//! trailing 15 minutes (11:25 to 11:40) hold M4 only: 4 + 300 = 304 tokens.
//! The whole 27th is 10628 headline tokens (cache reads left out).

mod common;

use std::path::PathBuf;

use common::pipeline::{Event, Rig};
use common::*;
use tokenmonk_lib::app::halo_percent;
use tokenmonk_lib::contracts::dashboard::BudgetBasis;
use tokenmonk_lib::contracts::limits::{LimitReading, LimitStatus, LimitWindow};
use tokenmonk_lib::contracts::{AchievementId, DashboardSnapshot, Mode};
use tokenmonk_lib::watcher::{discover, SnapshotCause};

fn fixture_files() -> Vec<PathBuf> {
    let files = discover(&fixtures_dir().join("projects")).expect("fixtures exist");
    assert_eq!(files.len(), 4, "the main set");
    files
}

/// A rig that has done the startup scan of the fixtures and published the
/// `ImportDone` snapshot at `now`.
fn loaded(mode: Mode, now: i64) -> (Rig, DashboardSnapshot) {
    let mut rig = Rig::new(mode);
    rig.scan(&fixture_files(), now, false);
    let s = rig.snap(now, SnapshotCause::ImportDone);
    (rig, s)
}

fn now() -> i64 {
    utc_ms(2026, 10, 28, 11, 40, 0)
}

fn window(utilization: f64, resets_at: i64) -> Option<LimitWindow> {
    Some(LimitWindow {
        utilization,
        resets_at: Some(resets_at),
    })
}

#[test]
fn today_session_and_burn_come_from_the_fixtures_without_cache_reads() {
    let (rig, s) = loaded(Mode::Max, now());

    assert_eq!(s.generated_at, now());
    assert_eq!(s.today.input, 8);
    assert_eq!(s.today.output, 1_000);
    assert_eq!(s.today.cache_creation, 1_000);
    assert_eq!(
        s.today.cache_read, 510_000,
        "reported, but not in the total"
    );
    assert_eq!(s.today.total, 2_008);

    // Burn rate: M4 only, cache read of 260000 left out.
    assert_eq!(s.burn_rate.window_minutes, 15);
    assert!((s.burn_rate.tokens_per_minute - 304.0 / 15.0).abs() < 1e-9);
    assert_eq!(s.burn_rate.limit_eta_minutes, None, "no limit data");

    // Session 3 is current (last message 10 minutes ago).
    let session = s.session.clone().expect("session 3 is active");
    assert_eq!(session.session_id, S3);
    assert_eq!(session.started_at, utc_ms(2026, 10, 27, 21, 59, 59) + 999);
    assert_eq!(session.last_activity_at, utc_ms(2026, 10, 28, 11, 30, 0));
    assert_eq!(session.tokens.input, 12);
    assert_eq!(session.tokens.output, 1_150);
    assert_eq!(session.tokens.cache_creation, 1_000);
    assert_eq!(session.tokens.total, 2_162);
    assert_eq!(session.tokens.cache_read, 920_000);

    assert_eq!(
        s.data.last_message_at,
        Some(utc_ms(2026, 10, 28, 11, 30, 0))
    );
    assert_eq!(s.limits.status, LimitStatus::Unavailable);
    assert!(s.daily_budget.is_none(), "Max it out has no history budget");

    // The app cache and the event stream carry the same snapshot.
    assert_eq!(rig.state.dashboard().as_ref(), Some(&s));
    assert!(rig.rec.events().contains(&Event::Dashboard));
}

#[test]
fn the_previous_day_is_complete_across_all_files_and_both_ends_of_the_day() {
    // Seen from the 29th, "today" is empty and the 28th is history.
    let (_, s) = loaded(Mode::Diy, utc_ms(2026, 10, 29, 9, 0, 0));
    assert_eq!(s.today.total, 0);
    assert_eq!(s.today.cache_read, 0);
    assert!(s.session.is_none(), "idle for more than an hour");
    assert_eq!(s.burn_rate.tokens_per_minute, 0.0);

    // Seen from the 28th, the 27th is 10628 headline tokens (sessions 1, 2,
    // the subagent lines and the two messages around the UTC midnight of +2).
    let (_, s) = loaded(Mode::Diy, utc_ms(2026, 10, 28, 12, 0, 0));
    let budget = s.daily_budget.expect("history of the 27th");
    assert_eq!(budget.basis, BudgetBasis::TokenHistory);
    assert_eq!(budget.allowance, 10_628.0);
    assert_eq!(budget.used_today, 2_008.0);
}

#[test]
fn halo_is_the_five_hour_percent_left_in_every_mode() {
    // Without a 5-hour reading the halo shows the dotted track only, in
    // every mode, whatever the token history says.
    for mode in [Mode::NoneYet, Mode::Max, Mode::Diy] {
        let (rig, s) = loaded(mode, now());
        assert_eq!(halo_percent(&s), None, "{mode:?}");
        assert!(rig.rec.events().contains(&Event::Halo(None, mode)));
    }

    // With a 5-hour reading of 20 % used, 80 % is left, in every mode, and
    // the sink is told the mode so it can pick the accent.
    for mode in [Mode::NoneYet, Mode::Max, Mode::Diy] {
        let (mut rig, _) = loaded(mode, now());
        rig.limit(Ok(LimitReading {
            five_hour: window(20.0, utc_ms(2026, 10, 28, 13, 40, 0)),
            weekly: None,
            fetched_at: now() - 60_000,
        }));
        let s = rig.snap(now(), SnapshotCause::Limits);
        assert_eq!(s.limits.status, LimitStatus::Ok);
        assert_eq!(halo_percent(&s), Some(80.0), "{mode:?}");
        assert!(rig.rec.events().contains(&Event::Halo(Some(80.0), mode)));
    }

    // Max keeps its ETA: 20 % over the 304 tokens since 08:40 at 304/15
    // tokens a minute is 1.33 %/min, so 60 minutes to go; float rounding may
    // land on 59.
    let (mut rig, _) = loaded(Mode::Max, now());
    rig.limit(Ok(LimitReading {
        five_hour: window(20.0, utc_ms(2026, 10, 28, 13, 40, 0)),
        weekly: None,
        fetched_at: now() - 60_000,
    }));
    let s = rig.snap(now(), SnapshotCause::Limits);
    let eta = s.burn_rate.limit_eta_minutes.expect("eta with a reading");
    assert!((59..=60).contains(&eta), "eta {eta}");

    // The gauge drains: a fuller window leaves less, and a window used up
    // (or over) clamps to empty, never negative.
    for (used, left) in [
        (0.0, 100.0),
        (99.4, 100.0 - 99.4),
        (100.0, 0.0),
        (130.0, 0.0),
    ] {
        let (mut rig, _) = loaded(Mode::Max, now());
        rig.limit(Ok(LimitReading {
            five_hour: window(used, utc_ms(2026, 10, 28, 13, 40, 0)),
            weekly: None,
            fetched_at: now() - 60_000,
        }));
        let s = rig.snap(now(), SnapshotCause::Limits);
        let got = halo_percent(&s).expect("five-hour reading");
        assert!((got - left).abs() < 1e-9, "used {used}: {got}");
    }
}

#[test]
fn the_do_it_yourself_budget_does_not_drive_the_halo() {
    // History budget: today against the average day of the last seven
    // (2008 of 10628 tokens). Without a limit reading the halo stays empty.
    let (rig, s) = loaded(Mode::Diy, utc_ms(2026, 10, 28, 12, 0, 0));
    assert!(s.daily_budget.is_some());
    assert_eq!(halo_percent(&s), None);
    assert!(rig.rec.events().contains(&Event::Halo(None, Mode::Diy)));

    // Weekly budget: weekly 30 % at the first reading of the day, 40 % now,
    // reset on the 30th: allowance (100 - 30) / 3 days, used 10 points. The
    // halo still follows the 5-hour window only (20 % used, 80 % left).
    let (mut rig, _) = loaded(Mode::Diy, now());
    let reset = utc_ms(2026, 10, 30, 18, 0, 0);
    rig.limit(Ok(LimitReading {
        five_hour: None,
        weekly: window(30.0, reset),
        fetched_at: utc_ms(2026, 10, 28, 0, 30, 0),
    }));
    let s = rig.snap(now(), SnapshotCause::Limits);
    assert_eq!(
        s.daily_budget.as_ref().unwrap().basis,
        BudgetBasis::WeeklyLimit
    );
    assert_eq!(halo_percent(&s), None, "weekly data alone gives no halo");

    rig.limit(Ok(LimitReading {
        five_hour: window(20.0, utc_ms(2026, 10, 28, 13, 40, 0)),
        weekly: window(40.0, reset),
        fetched_at: now() - 5 * 60_000,
    }));
    let s = rig.snap(now(), SnapshotCause::Limits);
    let budget = s.daily_budget.clone().expect("weekly budget");
    assert_eq!(budget.basis, BudgetBasis::WeeklyLimit);
    assert_eq!(budget.days_left, 3);
    assert!((budget.allowance - 70.0 / 3.0).abs() < 1e-9);
    assert!((budget.used_today - 10.0).abs() < 1e-9);
    assert_eq!(halo_percent(&s), Some(80.0));
    assert!(rig
        .rec
        .events()
        .contains(&Event::Halo(Some(80.0), Mode::Diy)));
}

#[test]
fn fixtures_alone_earn_first_blood_and_nothing_else() {
    // 10628 and 2008 headline tokens are both under one 100k block, there is
    // no message between 02:00 and 04:00 and no limit reading.
    let (rig, _) = loaded(Mode::Max, now());
    let game = rig.state.game().expect("evaluated at import done");
    assert!(game.retro_done);
    assert_eq!(game.xp_total, 25);
    let unlocked: Vec<_> = game
        .achievements
        .iter()
        .filter(|a| a.unlocked_at.is_some())
        .collect();
    assert_eq!(unlocked.len(), 1);
    assert_eq!(unlocked[0].id, AchievementId::FirstBlood);
    assert_eq!(
        unlocked[0].unlocked_at,
        Some(utc_ms(2026, 10, 27, 9, 0, 5) + 100)
    );
    assert_eq!(rig.xp_total(), 25);
}
