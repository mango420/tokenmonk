//! End to end: synthetic log files -> ingest -> snapshot -> game, through the
//! real `AppPublisher` with a recording sink. Covers the retroactive replay
//! (right numbers, quiet), live unlocks (emitted once, also across restarts)
//! and the agreement of replay and step by step play.
//!
//! The history (Max it out, UTC, one message per line unless noted). Every
//! message also carries 5,000,000 cache-read tokens, which must never count.
//! Max bands per 100k block: 10 XP up to 2M tokens a day, 2 XP up to 5M, 0
//! above; plus 10 XP for every active day (a `day` row).
//!
//! | day   | at    | headline  | blocks | block XP | day XP |
//! |-------|-------|-----------|--------|----------|--------|
//! | 10-20 | 03:10 |   250,000 | 2      | 20       | 10     |
//! | 10-21 | 10:00 | 2,500,000 | 25     | 200 + 10 | 10     |
//! | 10-22 | 10:00 | 1,000,000 |        |          |        |
//! | 10-22 | 15:00 | 1,100,000 | 21     | 200 + 2  | 10     |
//! | 10-24 | 11:00 |    99,999 | 0      | 0        | 10     |
//! | 10-25 | 11:00 |   100,000 | 1      | 10       | 10     |
//! | 10-28 | 09:00 |   350,000 | 3      | 30       | 10     |
//!
//! Token XP 472, day XP 60. First Blood (10 XP) and Night Owl (50 XP, the
//! 03:10 message) unlock at that message: 60 XP. Total 592. Level spans are
//! 200, 320, 440: level 3 starts at 520, so 72 XP into a span of 440.

mod common;

use std::path::PathBuf;

use common::pipeline::*;
use common::*;
use tokenmonk_lib::contracts::{AchievementId, GameState, Mode};
use tokenmonk_lib::watcher::SnapshotCause;

/// `(day of October 2026, hour, minute, headline tokens)`.
const HISTORY: [(u32, u32, u32, u64); 7] = [
    (20, 3, 10, 250_000),
    (21, 10, 0, 2_500_000),
    (22, 10, 0, 1_000_000),
    (22, 15, 0, 1_100_000),
    (24, 11, 0, 99_999),
    (25, 11, 0, 100_000),
    (28, 9, 0, 350_000),
];

const XP_TOTAL: u64 = 592;

fn replay_now() -> i64 {
    utc_ms(2026, 10, 28, 12, 0, 0)
}

fn lines_for(day: u32) -> Vec<String> {
    HISTORY
        .iter()
        .enumerate()
        .filter(|(_, (d, ..))| *d == day)
        .map(|(i, (d, h, m, tokens))| {
            heavy_line(i as u32 + 1, utc_ms(2026, 10, *d, *h, *m, 0), *tokens)
        })
        .collect()
}

fn history_days() -> Vec<u32> {
    let mut days: Vec<u32> = HISTORY.iter().map(|(d, ..)| *d).collect();
    days.dedup();
    days
}

/// A log folder holding the whole history, and its one file.
fn full_history() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let file = session_file(dir.path(), SESSION_HEAVY);
    for day in history_days() {
        append_lines(&file, &lines_for(day));
    }
    (dir, file)
}

/// The app after its first start over the whole history (the replay).
fn replayed(mode: Mode) -> (tempfile::TempDir, PathBuf, Rig) {
    let (dir, file) = full_history();
    let mut rig = Rig::new(mode);
    rig.scan(std::slice::from_ref(&file), replay_now(), false);
    rig.snap(replay_now(), SnapshotCause::ImportDone);
    (dir, file, rig)
}

fn unlocked(game: &GameState) -> Vec<(AchievementId, i64)> {
    game.achievements
        .iter()
        .filter_map(|a| a.unlocked_at.map(|at| (a.id, at)))
        .collect()
}

#[test]
fn replay_awards_the_expected_xp_and_level() {
    let (_dir, _file, rig) = replayed(Mode::Max);

    let game = rig.state.game().expect("evaluated at import done");
    assert!(game.retro_done);
    assert_eq!(game.mode, Mode::Max);
    assert_eq!(game.xp_total, XP_TOTAL);
    assert_eq!(game.level, 3);
    assert_eq!(game.xp_into_level, 72);
    assert_eq!(game.xp_for_level, 440);
    assert!((game.progress - 72.0 / 440.0).abs() < 1e-12);
    assert_eq!(game.title, "Token Tourist");
    assert_eq!(game.daily_xp_preview, None, "Max it out has no preview");

    // The achievements unlock at the 03:10 message (not at replay time).
    let first_message = utc_ms(2026, 10, 20, 3, 10, 0);
    assert_eq!(
        unlocked(&game),
        [
            (AchievementId::FirstBlood, first_message),
            (AchievementId::NightOwl, first_message)
        ]
    );

    // The ledger: 2 + 25 + 21 + 0 + 1 + 3 token blocks, six day bonuses.
    let ledger = rig.ledger();
    let tokens: Vec<_> = ledger.iter().filter(|(k, ..)| k == "tokens").collect();
    assert_eq!(tokens.len(), 52);
    assert_eq!(tokens.iter().map(|(_, _, a)| *a).sum::<i64>(), 472);
    let has =
        |kind: &str, reference: &str| ledger.iter().any(|(k, r, _)| k == kind && r == reference);
    let amount = |kind: &str, reference: &str| {
        ledger
            .iter()
            .find(|(k, r, _)| k == kind && r == reference)
            .map(|(_, _, a)| *a)
    };
    assert!(has("tokens", "2026-10-20/2") && !has("tokens", "2026-10-20/3"));
    assert_eq!(amount("tokens", "2026-10-21/20"), Some(10));
    assert_eq!(amount("tokens", "2026-10-21/21"), Some(2));
    assert_eq!(amount("tokens", "2026-10-21/25"), Some(2));
    assert!(!has("tokens", "2026-10-21/26"));
    assert_eq!(amount("tokens", "2026-10-22/21"), Some(2));
    assert!(!has("tokens", "2026-10-22/22"));
    assert!(!ledger
        .iter()
        .any(|(k, r, _)| k == "tokens" && r.starts_with("2026-10-24")));
    assert!(has("tokens", "2026-10-25/1") && !has("tokens", "2026-10-25/2"));
    assert!(has("tokens", "2026-10-28/3") && !has("tokens", "2026-10-28/4"));
    // The active-day bonus: one `day` row per day with a counted reply, even
    // the one token short of a block.
    for day in [20, 21, 22, 24, 25, 28] {
        assert_eq!(amount("day", &format!("2026-10-{day}")), Some(10));
    }
    assert_eq!(ledger.iter().filter(|(k, ..)| k == "day").count(), 6);
    assert!(ledger.contains(&("achievement".into(), "firstBlood".into(), 10)));
    assert!(ledger.contains(&("achievement".into(), "nightOwl".into(), 50)));
    assert_eq!(ledger.len(), 60);
    assert_eq!(rig.xp_total(), XP_TOTAL as i64);
}

#[test]
fn replay_is_marked_retroactive_and_quiet() {
    let (_dir, _file, rig) = replayed(Mode::Max);

    // Not one toast, not one level-up; a single game update closes the replay.
    assert!(rig.rec.achievements().is_empty());
    assert!(rig.rec.level_ups().is_empty());
    assert_eq!(
        rig.rec
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Game { .. }))
            .count(),
        1
    );
    assert!(rig.rec.events().contains(&Event::Game {
        retro_done: true,
        level: 3
    }));

    // Stored as retroactive, shown as retroactive.
    assert!(rig.db.xp_events().unwrap().iter().all(|e| e.retroactive));
    assert!(rig.db.achievements().unwrap().iter().all(|a| a.retroactive));
    let game = rig.state.game().unwrap();
    assert_eq!(game.recent_unlocks.len(), 2);
    assert!(game.recent_unlocks.iter().all(|u| u.retroactive));
}

#[test]
fn replay_runs_once_even_across_restarts_and_ticks() {
    let (_dir, file, mut rig) = replayed(Mode::Max);
    let ledger = rig.ledger();

    rig.snap(replay_now() + 60_000, SnapshotCause::Tick);
    rig.snap(replay_now() + 120_000, SnapshotCause::Requested);
    assert_eq!(rig.ledger(), ledger);

    // A restart rescans nothing new (offsets are stored) and replays nothing.
    rig.restart();
    rig.scan(&[file], replay_now() + 180_000, false);
    rig.snap(replay_now() + 180_000, SnapshotCause::ImportDone);
    assert_eq!(rig.ledger(), ledger);
    assert_eq!(rig.xp_total(), XP_TOTAL as i64);
    assert!(rig.rec.achievements().is_empty());
    assert!(rig.rec.level_ups().is_empty());
    assert!(rig.db.xp_events().unwrap().iter().all(|e| e.retroactive));
}

#[test]
fn history_waits_for_the_mode_and_is_then_replayed_quietly() {
    // First start, onboarding not done: usage is read, nothing is awarded.
    let (_dir, file) = full_history();
    let mut rig = Rig::new(Mode::NoneYet);
    rig.scan(std::slice::from_ref(&file), replay_now(), false);
    rig.snap(replay_now(), SnapshotCause::ImportDone);
    let game = rig.state.game().unwrap();
    assert_eq!((game.xp_total, game.level, game.retro_done), (0, 1, false));
    assert_eq!(rig.xp_total(), 0);

    // The user picks a mode; the next snapshot replays the history.
    rig.set_mode(Mode::Max);
    rig.snap(replay_now() + 1_000, SnapshotCause::Requested);
    let game = rig.state.game().unwrap();
    assert!(game.retro_done);
    assert_eq!(game.xp_total, XP_TOTAL);
    assert_eq!(game.level, 3);
    assert!(rig.rec.achievements().is_empty());
    assert!(rig.rec.level_ups().is_empty());
    assert!(rig.db.xp_events().unwrap().iter().all(|e| e.retroactive));
}

#[test]
fn live_unlocks_are_emitted_once_and_never_repeated() {
    // The app starts with no history: the (empty) replay is done at once.
    let dir = tempfile::tempdir().unwrap();
    let file = session_file(dir.path(), SESSION_HEAVY);
    let mut rig = Rig::new(Mode::Max);
    rig.snap(utc_ms(2026, 10, 20, 2, 0, 0), SnapshotCause::ImportDone);
    assert!(rig.state.game().unwrap().retro_done);
    assert!(rig.rec.achievements().is_empty());

    // Day by day, as the user would have lived it.
    let mut xp_after = Vec::new();
    for day in history_days() {
        append_lines(&file, &lines_for(day));
        let now = if day == 28 {
            replay_now()
        } else {
            utc_ms(2026, 10, day, 20, 0, 0)
        };
        rig.scan(std::slice::from_ref(&file), now, true);
        rig.snap(now, SnapshotCause::Files);
        xp_after.push((day, rig.xp_total()));
    }
    assert_eq!(
        xp_after,
        [
            (20, 90),
            (21, 310),
            (22, 522),
            (24, 532),
            (25, 552),
            (28, 592)
        ]
    );

    // First Blood and Night Owl, once each, live, in display order.
    assert_eq!(rig.rec.achievements(), ["firstBlood", "nightOwl"]);
    assert!(rig.rec.events().iter().all(|e| match e {
        Event::Achievement { retroactive, .. } => !retroactive,
        _ => true,
    }));
    // The unlock events come before the state they are part of.
    let events = rig.rec.events();
    let first_unlock = events
        .iter()
        .position(|e| matches!(e, Event::Achievement { .. }))
        .unwrap();
    // Within the same publisher pass (before the next dashboard event).
    let pass_end = events[first_unlock..]
        .iter()
        .position(|e| matches!(e, Event::Dashboard))
        .map_or(events.len(), |i| first_unlock + i);
    assert!(events[first_unlock..pass_end]
        .iter()
        .any(|e| matches!(e, Event::Game { .. })));

    // Two level-ups: 310 XP on the 21st crosses 200, 522 XP on the 22nd
    // crosses 520.
    assert_eq!(
        rig.rec.level_ups(),
        [
            Event::LevelUp {
                level: 2,
                previous: 1,
                retroactive: false
            },
            Event::LevelUp {
                level: 3,
                previous: 2,
                retroactive: false
            }
        ]
    );

    // More snapshots of every kind add nothing.
    let before = rig.rec.events().len();
    for (i, cause) in [
        SnapshotCause::Tick,
        SnapshotCause::Files,
        SnapshotCause::Limits,
        SnapshotCause::Requested,
    ]
    .into_iter()
    .enumerate()
    {
        rig.snap(replay_now() + 60_000 * (i as i64 + 1), cause);
    }
    assert_eq!(rig.rec.achievements().len(), 2);
    assert_eq!(rig.rec.level_ups().len(), 2);
    assert_eq!(rig.xp_total(), XP_TOTAL as i64);
    assert!(rig.rec.events()[before..]
        .iter()
        .all(|e| !matches!(e, Event::Achievement { .. } | Event::LevelUp { .. })));

    // Neither a restart with the full scan, nor a rescan from scratch of
    // the whole file (offsets lost) announces anything again.
    rig.restart();
    rig.scan(std::slice::from_ref(&file), replay_now() + 600_000, false);
    rig.snap(replay_now() + 600_000, SnapshotCause::ImportDone);
    rig.db
        .conn()
        .execute("DELETE FROM source_files", [])
        .unwrap();
    rig.scan(std::slice::from_ref(&file), replay_now() + 700_000, true);
    rig.snap(replay_now() + 700_000, SnapshotCause::Files);
    assert!(rig.rec.achievements().is_empty());
    assert!(rig.rec.level_ups().is_empty());
    assert_eq!(rig.xp_total(), XP_TOTAL as i64);
}

#[test]
fn step_by_step_play_ends_where_the_replay_does() {
    let (_dir, _file, replay) = replayed(Mode::Max);

    let dir = tempfile::tempdir().unwrap();
    let file = session_file(dir.path(), SESSION_HEAVY);
    let mut live = Rig::new(Mode::Max);
    live.snap(utc_ms(2026, 10, 20, 2, 0, 0), SnapshotCause::ImportDone);
    for day in history_days() {
        append_lines(&file, &lines_for(day));
        let now = if day == 28 {
            replay_now()
        } else {
            utc_ms(2026, 10, day, 20, 0, 0)
        };
        live.scan(std::slice::from_ref(&file), now, true);
        live.snap(now, SnapshotCause::Files);
    }

    assert_eq!(live.ledger(), replay.ledger());
    let (a, b) = (live.state.game().unwrap(), replay.state.game().unwrap());
    assert_eq!(a.xp_total, b.xp_total);
    assert_eq!((a.level, a.xp_into_level), (b.level, b.xp_into_level));
    assert_eq!(a.achievements, b.achievements);
}

#[test]
fn a_busy_day_pays_a_soft_rate_up_to_five_million_and_nothing_above() {
    let (_dir, file, mut rig) = replayed(Mode::Max);

    // 3,000,000 more tokens today on top of 350,000: 3.35M, 33 blocks.
    append_lines(
        &file,
        &[heavy_line(100, utc_ms(2026, 10, 28, 10, 0, 0), 3_000_000)],
    );
    let now = utc_ms(2026, 10, 28, 10, 30, 0);
    rig.scan(std::slice::from_ref(&file), now, true);
    rig.snap(now, SnapshotCause::Files);
    // Blocks 4 to 20 are new at 10 XP, blocks 21 to 33 at 2 XP; the day
    // bonus was paid already.
    assert_eq!(rig.xp_total(), XP_TOTAL as i64 + 170 + 26);
    assert!(rig.rec.achievements().is_empty());

    // 5,000,000 more, same day: 8.35M. Blocks 34 to 50 pay 2 XP, above 5M
    // (block 50) nothing.
    append_lines(
        &file,
        &[heavy_line(101, utc_ms(2026, 10, 28, 11, 0, 0), 5_000_000)],
    );
    rig.scan(std::slice::from_ref(&file), now + 1_800_000, true);
    rig.snap(now + 1_800_000, SnapshotCause::Files);
    assert_eq!(rig.xp_total(), XP_TOTAL as i64 + 170 + 26 + 34);

    // Still more: the day is at its ceiling (270 XP) and stays there.
    append_lines(
        &file,
        &[heavy_line(102, utc_ms(2026, 10, 28, 12, 0, 0), 4_000_000)],
    );
    rig.scan(std::slice::from_ref(&file), now + 3_600_000, true);
    rig.snap(now + 3_600_000, SnapshotCause::Files);
    assert_eq!(rig.xp_total(), XP_TOTAL as i64 + 170 + 26 + 34);
    let day_xp: i64 = rig
        .ledger()
        .into_iter()
        .filter(|(k, r, _)| (k == "tokens" || k == "day") && r.starts_with("2026-10-28"))
        .map(|(.., a)| a)
        .sum();
    assert_eq!(day_xp, 270);
}

#[test]
fn the_title_follows_the_mode_at_every_level() {
    // Pre-onboarding: level 1, no history read yet, the first Max title.
    let (_dir, file) = full_history();
    let mut rig = Rig::new(Mode::NoneYet);
    rig.scan(std::slice::from_ref(&file), replay_now(), false);
    rig.snap(replay_now(), SnapshotCause::ImportDone);
    let game = rig.state.game().unwrap();
    assert_eq!((game.level, game.title.as_str()), (1, "Token Tourist"));

    // Max it out, level 3 after the replay (tier 1 to 4).
    rig.set_mode(Mode::Max);
    rig.snap(replay_now() + 1_000, SnapshotCause::Requested);
    let game = rig.state.game().unwrap();
    assert_eq!((game.level, game.title.as_str()), (3, "Token Tourist"));

    // Do it yourself: the other title set. Days are paid by the budget they
    // used (150 XP up to 50 %, 0 above 100 %, 50 unmeasured), plus 160 XP of
    // achievements.
    let (_dir, _file, rig) = replayed(Mode::Diy);
    let game = rig.state.game().unwrap();
    assert!(game.retro_done);
    assert_eq!(game.xp_total, 510);
    assert_eq!((game.level, game.title.as_str()), (2, "Budget Curious"));
}

#[test]
fn night_owl_covers_midnight_to_four() {
    // A lone message at 01:30 UTC (the test zone) is inside the 00:00-04:00
    // window; before the refresh only 02:00-04:00 counted.
    let dir = tempfile::tempdir().unwrap();
    let file = session_file(dir.path(), SESSION_HEAVY);
    let mut rig = Rig::new(Mode::Max);
    rig.snap(utc_ms(2026, 10, 20, 0, 30, 0), SnapshotCause::ImportDone);
    let at = utc_ms(2026, 10, 20, 1, 30, 0);
    append_lines(&file, &[heavy_line(1, at, 50_000)]);
    rig.scan(std::slice::from_ref(&file), at + 1_000, true);
    rig.snap(at + 1_000, SnapshotCause::Files);
    assert_eq!(rig.rec.achievements(), ["firstBlood", "nightOwl"]);

    // 04:00 sharp is outside it.
    let dir = tempfile::tempdir().unwrap();
    let file = session_file(dir.path(), SESSION_HEAVY);
    let mut rig = Rig::new(Mode::Max);
    rig.snap(utc_ms(2026, 10, 20, 3, 0, 0), SnapshotCause::ImportDone);
    let at = utc_ms(2026, 10, 20, 4, 0, 0);
    append_lines(&file, &[heavy_line(1, at, 50_000)]);
    rig.scan(std::slice::from_ref(&file), at + 1_000, true);
    rig.snap(at + 1_000, SnapshotCause::Files);
    assert_eq!(rig.rec.achievements(), ["firstBlood"]);
}

#[test]
fn an_old_ledger_is_recounted_once_keeping_achievements() {
    use chrono::{NaiveDate, Utc};
    use tokenmonk_lib::game::{
        recount_if_outdated, run_evaluation, AchievementRecord, DayFacts, Facts, LedgerEntry,
        MemoryStore,
    };

    let day = |d: u32| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
    let usage = |d: u32, tokens: u64| {
        DayFacts::from_usage(
            day(d),
            [(utc_ms(2026, 10, d, 10, 0, 0), tokens)],
            None,
            &Utc,
        )
    };
    let facts = Facts {
        days: vec![usage(20, 250_000), usage(21, 2_500_000), usage(22, 0)],
        ..Facts::default()
    };
    let now = utc_ms(2026, 10, 22, 12, 0, 0);
    let row = |kind: &str, reference: String, amount: u32| LedgerEntry {
        earned_at: utc_ms(2026, 10, 20, 10, 0, 0),
        mode: Mode::Max,
        kind: kind.into(),
        reference,
        amount,
        retroactive: true,
    };

    // The ledger as the old rules left it: 10 XP a block capped at 20 a day,
    // no day rows, First Blood unlocked (10 XP).
    let mut old = vec![row("achievement", "firstBlood".into(), 10)];
    old.extend((1..=2).map(|k| row("tokens", format!("2026-10-20/{k}"), 10)));
    old.extend((1..=20).map(|k| row("tokens", format!("2026-10-21/{k}"), 10)));
    let old_total: u64 = old.iter().map(|e| u64::from(e.amount)).sum();
    assert_eq!(old_total, 230);
    let mut store = MemoryStore::default();
    store.data.ledger = old;
    store.data.achievements = vec![AchievementRecord {
        id: AchievementId::FirstBlood,
        unlocked_at: Some(utc_ms(2026, 10, 20, 10, 0, 0)),
        retroactive: true,
        current: None,
        target: None,
        updated_at: utc_ms(2026, 10, 20, 10, 0, 0),
    }];
    store.data.retro_done = true;
    store.rules_version = Some(1);
    let achievements = store.data.achievements.clone();

    // Recounted exactly once.
    assert!(recount_if_outdated(&mut store).unwrap());
    assert!(!store.data.retro_done, "the next evaluation is a replay");
    assert_eq!(store.data.achievements, achievements);
    let after = store.data.clone();
    assert!(!recount_if_outdated(&mut store).unwrap());
    assert_eq!(store.data, after, "idempotent");

    // The replay under the new rules: retroactive, no unlock, no level-up
    // announcement.
    let ev = run_evaluation(&mut store, &facts, Mode::Max, now, &Utc, true).unwrap();
    assert!(ev.unlocks.is_empty(), "achievements are kept, not re-won");
    assert!(ev.ledger.iter().all(|e| e.retroactive));
    assert!(store.data.retro_done);
    // 20 + 10 (the 20th), 210 + 10 (the 21st), plus First Blood.
    assert_eq!(ev.state.xp_total, 30 + 220 + 10);
    assert!(ev.state.xp_total >= old_total, "never lowers the total");
    assert_eq!((ev.state.level, ev.state.xp_into_level), (2, 60));
    assert!(ev.state.retro_done);

    // A second replay, and a second recount, change nothing.
    let settled = store.data.clone();
    let again = run_evaluation(&mut store, &facts, Mode::Max, now, &Utc, true).unwrap();
    assert!(again.ledger.is_empty() && again.unlocks.is_empty());
    assert!(!recount_if_outdated(&mut store).unwrap());
    assert_eq!(store.data, settled);
}
