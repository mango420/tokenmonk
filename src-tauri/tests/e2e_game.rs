//! End to end: synthetic log files -> ingest -> snapshot -> game, through the
//! real `AppPublisher` with a recording sink. Covers the retroactive replay
//! (right numbers, quiet), live unlocks (emitted once, also across restarts)
//! and the agreement of replay and step by step play.
//!
//! The history (Max it out, UTC, one message per line unless noted). Every
//! message also carries 5,000,000 cache-read tokens, which must never count.
//!
//! | day   | at    | headline  | blocks (100k, max 20 a day) | XP |
//! |-------|-------|-----------|-----------------------------|----|
//! | 10-20 | 03:10 |   250,000 | 2                           | 20 |
//! | 10-21 | 10:00 | 2,500,000 | 25, capped at 20            | 200 |
//! | 10-22 | 10:00 | 1,000,000 |                             |    |
//! | 10-22 | 15:00 | 1,100,000 | 21, capped at 20            | 200 |
//! | 10-24 | 11:00 |    99,999 | 0 (one token short)         | 0 |
//! | 10-25 | 11:00 |   100,000 | 1 (exactly one block)       | 10 |
//! | 10-28 | 09:00 |   350,000 | 3 (today counts in Max)     | 30 |
//!
//! Token XP 460. First Blood (25 XP) and Night Owl (50 XP, the 03:10 message)
//! unlock at that message: 75 XP. Total 535. Level 1 spans 470 XP, level 2
//! spans 700, so the player is level 2 with 65 XP into it.

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

const XP_TOTAL: u64 = 535;

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
    assert_eq!(game.level, 2);
    assert_eq!(game.xp_into_level, 65);
    assert_eq!(game.xp_for_level, 700);
    assert!((game.progress - 65.0 / 700.0).abs() < 1e-12);
    assert_eq!(game.title, "Token Sprout");
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

    // The ledger: 2 + 20 + 20 + 0 + 1 + 3 token blocks and two bonuses.
    let ledger = rig.ledger();
    let tokens: Vec<_> = ledger.iter().filter(|(k, ..)| k == "tokens").collect();
    assert_eq!(tokens.len(), 46);
    assert!(tokens.iter().all(|(_, _, amount)| *amount == 10));
    let has =
        |kind: &str, reference: &str| ledger.iter().any(|(k, r, _)| k == kind && r == reference);
    assert!(has("tokens", "2026-10-20/2") && !has("tokens", "2026-10-20/3"));
    assert!(has("tokens", "2026-10-21/20") && !has("tokens", "2026-10-21/21"));
    assert!(has("tokens", "2026-10-22/20") && !has("tokens", "2026-10-22/21"));
    assert!(!ledger.iter().any(|(_, r, _)| r.starts_with("2026-10-24")));
    assert!(has("tokens", "2026-10-25/1") && !has("tokens", "2026-10-25/2"));
    assert!(has("tokens", "2026-10-28/3") && !has("tokens", "2026-10-28/4"));
    assert!(ledger.contains(&("achievement".into(), "firstBlood".into(), 25)));
    assert!(ledger.contains(&("achievement".into(), "nightOwl".into(), 50)));
    assert_eq!(ledger.len(), 48);
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
        level: 2
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
    assert_eq!(game.level, 2);
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
            (20, 95),
            (21, 295),
            (22, 495),
            (24, 495),
            (25, 505),
            (28, 535)
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

    // One level-up: 495 XP on the 22nd crosses 470.
    assert_eq!(
        rig.rec.level_ups(),
        [Event::LevelUp {
            level: 2,
            previous: 1,
            retroactive: false
        }]
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
    assert_eq!(rig.rec.level_ups().len(), 1);
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
fn a_busy_day_stops_paying_at_twenty_blocks_and_the_cap_is_idempotent() {
    let (_dir, file, mut rig) = replayed(Mode::Max);

    // 3,000,000 more tokens today on top of 350,000: far past 2,000,000.
    append_lines(
        &file,
        &[heavy_line(100, utc_ms(2026, 10, 28, 10, 0, 0), 3_000_000)],
    );
    let now = utc_ms(2026, 10, 28, 10, 30, 0);
    rig.scan(std::slice::from_ref(&file), now, true);
    rig.snap(now, SnapshotCause::Files);
    // Blocks 4 to 20 of the 28th are new: 17 x 10 XP.
    assert_eq!(rig.xp_total(), XP_TOTAL as i64 + 170);
    assert!(rig.rec.achievements().is_empty());

    // Even more tokens, same day: nothing further.
    append_lines(
        &file,
        &[heavy_line(101, utc_ms(2026, 10, 28, 11, 0, 0), 5_000_000)],
    );
    rig.scan(std::slice::from_ref(&file), now + 1_800_000, true);
    rig.snap(now + 1_800_000, SnapshotCause::Files);
    assert_eq!(rig.xp_total(), XP_TOTAL as i64 + 170);
    let day_rows = rig
        .ledger()
        .into_iter()
        .filter(|(k, r, _)| k == "tokens" && r.starts_with("2026-10-28/"))
        .count();
    assert_eq!(day_rows, 20);
}
