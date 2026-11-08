use std::collections::BTreeMap;

use chrono::{Days, FixedOffset, NaiveDate, TimeZone, Utc};

use super::constants::*;
use super::levels::{level_info, xp_for_level};
use super::*;
use crate::contracts::{AchievementId, Mode};

const HOUR: i64 = 3_600_000;
const DAY: i64 = 24 * HOUR;

fn ms(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
    Utc.with_ymd_and_hms(y, mo, d, h, mi, 0)
        .unwrap()
        .timestamp_millis()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn day(d: NaiveDate, tokens: u64, budget: Option<f64>) -> DayFacts {
    DayFacts {
        day: d,
        tokens,
        first_at: (tokens > 0).then(|| day_start_ms(d, &Utc) + 10 * HOUR),
        night_at: None,
        budget_used_percent: budget,
    }
}

fn facts(days: Vec<DayFacts>) -> Facts {
    Facts {
        days,
        ..Facts::default()
    }
}

fn apply(store: &mut MemoryStore, f: &Facts, mode: Mode, now: i64, retro: bool) -> Evaluation {
    run_evaluation(store, f, mode, now, &Utc, retro).unwrap()
}

fn unlocked(ev: &Evaluation) -> Vec<AchievementId> {
    ev.unlocks.iter().map(|u| u.id).collect()
}

fn limit(at: i64, pct: f64, resets: i64) -> LimitFact {
    LimitFact {
        at,
        five_hour_pct: Some(pct),
        five_hour_resets_at: Some(resets),
    }
}

#[test]
fn max_awards_token_blocks_once() {
    let mut store = MemoryStore::default();
    let d = date(2026, 10, 28);
    let now = ms(2026, 10, 28, 15, 0);
    let f = facts(vec![day(d, 250_000, None)]);

    let ev = apply(&mut store, &f, Mode::Max, now, false);
    let blocks: Vec<_> = ev.ledger.iter().filter(|e| e.kind == KIND_TOKENS).collect();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].reference, "2026-10-28/1");
    assert_eq!(blocks[0].amount, XP_PER_BLOCK);
    assert_eq!(blocks[0].earned_at, day_start_ms(d, &Utc));
    assert_eq!(blocks[0].mode, Mode::Max);

    // Same facts again: nothing new.
    let again = apply(&mut store, &f, Mode::Max, now, false);
    assert!(again.ledger.is_empty());
    assert!(again.unlocks.is_empty());
    assert_eq!(again.state.xp_total, ev.state.xp_total);

    // More tokens later the same day: only the new blocks.
    let f = facts(vec![day(d, 410_000, None)]);
    let more = apply(&mut store, &f, Mode::Max, now + HOUR, false);
    assert_eq!(more.ledger.len(), 2);
}

fn max_day_xp(tokens: u64) -> u32 {
    let ev = evaluate(
        &GameData::default(),
        &facts(vec![day(date(2026, 10, 28), tokens, None)]),
        Mode::Max,
        ms(2026, 10, 29, 1, 0),
        &Utc,
        false,
    );
    ev.ledger
        .iter()
        .filter(|e| e.kind != KIND_ACHIEVEMENT)
        .map(|e| e.amount)
        .sum()
}

#[test]
fn max_bands_and_active_day_bonus() {
    assert_eq!(max_day_xp(1), MAX_ACTIVE_DAY_XP, "small day still counts");
    assert_eq!(max_day_xp(99_999), 10);
    assert_eq!(max_day_xp(100_000), 10 + 10);
    assert_eq!(max_day_xp(2_000_000), 200 + 10);
    assert_eq!(max_day_xp(2_100_000), 200 + 2 + 10);
    assert_eq!(max_day_xp(5_000_000), 200 + 60 + 10);
    assert_eq!(max_day_xp(5_100_000), 270);
    assert_eq!(max_day_xp(50_000_000), 270);
}

#[test]
fn diy_edges() {
    assert_eq!(diy_day_xp(0.0), 150);
    assert_eq!(diy_day_xp(50.0), 150);
    assert_eq!(diy_day_xp(75.0), 90);
    assert_eq!(diy_day_xp(100.0), 30);
    assert_eq!(diy_day_xp(100.1), 0);
}

fn old_history() -> Facts {
    facts(vec![
        day(date(2026, 10, 20), 3_000_000, None),
        day(date(2026, 10, 21), 400_000, None),
    ])
}

#[test]
fn recount_runs_once_and_matches_a_fresh_replay() {
    let now = ms(2026, 10, 28, 12, 0);
    let mut store = MemoryStore::default();
    apply(&mut store, &old_history(), Mode::Max, now, true);
    // Build the legacy state: old-rules rows, achievements, outdated version.
    let kept: Vec<_> = store
        .data
        .ledger
        .iter()
        .filter(|e| e.kind == KIND_ACHIEVEMENT)
        .cloned()
        .collect();
    assert!(!kept.is_empty());
    store.data.ledger.retain(|e| e.kind == KIND_ACHIEVEMENT);
    for (d, n) in [("2026-10-20", 20), ("2026-10-21", 4)] {
        for k in 1..=n {
            store.data.ledger.push(LedgerEntry {
                earned_at: 1,
                mode: Mode::Diy,
                kind: KIND_TOKENS.into(),
                reference: format!("{d}/{k}"),
                amount: 10,
                retroactive: true,
            });
        }
    }
    store.rules_version = Some(1);

    assert!(recount_if_outdated(&mut store).unwrap());
    assert!(!store.data.retro_done);
    assert_eq!(store.rules_version, Some(RULES_VERSION));
    assert!(store
        .data
        .ledger
        .iter()
        .all(|e| e.kind != KIND_TOKENS && e.kind != KIND_DAY));
    let snapshot = store.data.clone();
    assert!(!recount_if_outdated(&mut store).unwrap(), "runs once");
    assert_eq!(store.data, snapshot);

    // Replay under the new rules; the days keep the mode they were earned in,
    // so a Max-mode run does not take them over from Diy.
    let ev = apply(&mut store, &old_history(), Mode::Max, now, true);
    assert!(
        ev.ledger
            .iter()
            .all(|e| e.kind != KIND_TOKENS && e.kind != KIND_DAY),
        "days belong to the recorded mode"
    );

    let ach: u32 = kept.iter().map(|e| e.amount).sum();
    let xp: u64 = store.data.ledger.iter().map(|e| u64::from(e.amount)).sum();
    assert_eq!(xp, u64::from(ach));
}

#[test]
fn recount_replay_equals_live_levelling() {
    let now = ms(2026, 10, 28, 12, 0);
    let mut recounted = MemoryStore::default();
    recounted.data.ledger.push(LedgerEntry {
        earned_at: 1,
        mode: Mode::Max,
        kind: KIND_TOKENS.into(),
        reference: "2026-10-20/1".into(),
        amount: 10,
        retroactive: true,
    });
    recounted.data.retro_done = true;
    recounted.rules_version = Some(0);
    recount_if_outdated(&mut recounted).unwrap();
    let ev = apply(&mut recounted, &old_history(), Mode::Max, now, true);
    assert!(ev.state.retro_done);

    let mut fresh = MemoryStore::default();
    let live = apply(&mut fresh, &old_history(), Mode::Max, now, false);
    assert_eq!(ev.state.xp_total, live.state.xp_total);
}

#[test]
fn none_yet_awards_nothing() {
    let f = facts(vec![day(date(2026, 10, 28), 900_000, None)]);
    let ev = evaluate(
        &GameData::default(),
        &f,
        Mode::NoneYet,
        ms(2026, 10, 28, 12, 0),
        &Utc,
        false,
    );
    assert!(ev.ledger.is_empty());
    assert!(ev.unlocks.is_empty());
    assert!(ev.level_up.is_none());
    assert_eq!(ev.state.level, 1);
}

#[test]
fn first_blood_and_night_owl_with_local_detail() {
    let tz = FixedOffset::east_opt(3600).unwrap();
    let d = date(2026, 10, 28);
    // 23:47 UTC is 00:47 local (the next day), inside the midnight window.
    let ts = ms(2026, 10, 27, 23, 47);
    assert!(!in_night_window(ms(2026, 10, 28, 4, 0), &Utc));
    assert!(in_night_window(ts, &tz));
    assert!(!in_night_window(ts, &Utc));
    let usage = [(ts, 1_000u64), (ms(2026, 10, 28, 9, 0), 500)];
    let f = facts(vec![DayFacts::from_usage(d, usage, None, &tz)]);
    // 03:59 and 04:00 local: the window ends at 04:00, exclusive.
    assert!(in_night_window(ms(2026, 10, 28, 2, 59), &tz));
    assert!(!in_night_window(ms(2026, 10, 28, 3, 0), &tz));

    let ev = evaluate(
        &GameData::default(),
        &f,
        Mode::Max,
        ms(2026, 10, 28, 12, 0),
        &tz,
        false,
    );
    assert_eq!(
        unlocked(&ev),
        vec![AchievementId::FirstBlood, AchievementId::NightOwl]
    );
    let owl = &ev.unlocks[1];
    assert_eq!(owl.detail.as_deref(), Some("Coding at 00:47"));
    assert_eq!(owl.xp, 50);
    assert_eq!(owl.unlocked_at, ts);
    assert!(!owl.retroactive);
    // Two achievements plus the Max active-day bonus.
    assert_eq!(ev.state.xp_total, 10 + 50 + MAX_ACTIVE_DAY_XP as u64);
    assert_eq!(ev.state.recent_unlocks.len(), 2);
}

#[test]
fn night_owl_is_max_only() {
    let mut d = day(date(2026, 10, 28), 1_000, Some(1.0));
    d.night_at = Some(ms(2026, 10, 28, 3, 0));
    let ev = evaluate(
        &GameData::default(),
        &facts(vec![d]),
        Mode::Diy,
        ms(2026, 10, 28, 12, 0),
        &Utc,
        false,
    );
    assert_eq!(unlocked(&ev), vec![AchievementId::FirstBlood]);
}

#[test]
fn rambo_and_so_close_from_limit_readings() {
    let now = ms(2026, 10, 28, 20, 0);
    let a_end = ms(2026, 10, 28, 5, 0);
    let b_end = ms(2026, 10, 28, 10, 0);
    let c_end = ms(2026, 10, 28, 23, 0);
    let f = Facts {
        limits: vec![
            // Window A: peaks at 99.4 and is over.
            limit(ms(2026, 10, 28, 2, 0), 40.0, a_end),
            limit(ms(2026, 10, 28, 3, 0), 99.4, a_end + 7),
            limit(ms(2026, 10, 28, 4, 0), 99.2, a_end),
            // Window B: hits the limit.
            limit(ms(2026, 10, 28, 8, 30), 100.0, b_end),
            // Window C: 99.5 but still open.
            limit(ms(2026, 10, 28, 19, 0), 99.5, c_end),
        ],
        ..Facts::default()
    };
    let ev = evaluate(&GameData::default(), &f, Mode::Max, now, &Utc, false);
    assert_eq!(
        unlocked(&ev),
        vec![AchievementId::SoClose, AchievementId::RateLimitRambo]
    );
    // So Close counts from the end of its window, Rambo from the reading.
    assert_eq!(ev.unlocks[0].unlocked_at, a_end + 7);
    assert_eq!(ev.unlocks[1].unlocked_at, ms(2026, 10, 28, 8, 30));

    // Once window C is over, it is a second candidate but So Close stays one.
    let mut prev = GameData::default();
    let mut store = MemoryStore::default();
    apply(&mut store, &f, Mode::Max, now, false);
    prev.clone_from(&store.data);
    let later = evaluate(&prev, &f, Mode::Max, c_end + HOUR, &Utc, false);
    assert!(later.unlocks.is_empty());
    assert!(later.ledger.is_empty());
}

#[test]
fn a_window_that_hit_the_limit_is_not_so_close() {
    let end = ms(2026, 10, 28, 5, 0);
    let f = Facts {
        limits: vec![
            limit(ms(2026, 10, 28, 3, 0), 99.5, end),
            limit(ms(2026, 10, 28, 4, 0), 100.0, end),
        ],
        ..Facts::default()
    };
    let ev = evaluate(
        &GameData::default(),
        &f,
        Mode::Max,
        ms(2026, 10, 28, 12, 0),
        &Utc,
        false,
    );
    assert_eq!(unlocked(&ev), vec![AchievementId::RateLimitRambo]);
}

#[test]
fn diy_awards_finished_days_by_budget_and_previews_today() {
    let mut store = MemoryStore::default();
    let today = date(2026, 10, 28);
    let now = ms(2026, 10, 28, 14, 0);
    let f = facts(vec![
        day(date(2026, 10, 26), 5_000, Some(23.0)),
        day(date(2026, 10, 27), 5_000, None),
        day(today, 5_000, Some(23.0)),
    ]);
    let ev = apply(&mut store, &f, Mode::Diy, now, false);
    let amounts: BTreeMap<_, _> = ev
        .ledger
        .iter()
        .filter(|e| e.kind == KIND_DAY)
        .map(|e| (e.reference.clone(), e.amount))
        .collect();
    assert_eq!(amounts.len(), 2, "today is not finished");
    assert_eq!(amounts["2026-10-26"], 150);
    assert_eq!(amounts["2026-10-27"], DIY_UNMEASURED_DAY_XP);
    assert_eq!(ev.state.daily_xp_preview, Some(150));

    let over = facts(vec![day(today, 5_000, Some(130.0))]);
    let ev = evaluate(&store.data, &over, Mode::Diy, now, &Utc, false);
    assert_eq!(ev.state.daily_xp_preview, Some(0));
    let none = facts(vec![day(today, 5_000, None)]);
    let ev = evaluate(&store.data, &none, Mode::Diy, now, &Utc, false);
    assert_eq!(ev.state.daily_xp_preview, None);
    let ev = evaluate(&store.data, &f, Mode::Max, now, &Utc, false);
    assert_eq!(ev.state.daily_xp_preview, None);
}

#[test]
fn a_day_stays_with_the_mode_that_earned_it() {
    let mut store = MemoryStore::default();
    let d = date(2026, 10, 27);
    let f = facts(vec![
        day(d, 300_000, Some(10.0)),
        day(date(2026, 10, 28), 0, None),
    ]);
    apply(&mut store, &f, Mode::Max, ms(2026, 10, 27, 23, 0), false);
    let before = store.data.ledger.len();
    // Switched to Do it yourself the next day: the finished day is not paid twice.
    let ev = apply(&mut store, &f, Mode::Diy, ms(2026, 10, 28, 9, 0), false);
    assert!(ev.ledger.iter().all(|e| e.kind != KIND_DAY));
    assert!(store.data.ledger.len() >= before);
}

#[test]
fn token_monk_is_one_finished_day_under_the_daily_budget() {
    let first = date(2026, 10, 1);
    let monk = |ev: &Evaluation| {
        ev.unlocks
            .iter()
            .find(|u| u.id == AchievementId::TokenMonk)
            .cloned()
    };

    // Today does not count, however calm it is so far.
    let mut store = MemoryStore::default();
    let now = day_start_ms(first, &Utc) + 12 * HOUR;
    let ev = apply(
        &mut store,
        &facts(vec![day(first, 800, Some(5.0))]),
        Mode::Diy,
        now,
        false,
    );
    assert!(monk(&ev).is_none());

    // A finished day without a budget, over budget, or idle does not count.
    for d in [
        day(first, 800, None),
        day(first, 800, Some(100.0)),
        day(first, 800, Some(130.0)),
        day(first, 0, Some(0.0)),
    ] {
        let mut store = MemoryStore::default();
        let now = day_start_ms(first + Days::new(1), &Utc) + HOUR;
        let ev = apply(&mut store, &facts(vec![d]), Mode::Diy, now, false);
        assert!(monk(&ev).is_none());
    }

    // One finished day at 99.9 % unlocks it at the end of that day.
    let mut store = MemoryStore::default();
    let days = vec![
        day(first, 800, Some(99.9)),
        day(first + Days::new(1), 800, Some(150.0)),
    ];
    let now = day_start_ms(first + Days::new(1), &Utc) + HOUR;
    let ev = apply(&mut store, &facts(days), Mode::Diy, now, false);
    let unlock = monk(&ev).expect("token monk");
    assert_eq!(unlock.xp, 150);
    assert_eq!(unlock.detail, None);
    assert_eq!(unlock.unlocked_at, day_start_ms(first + Days::new(1), &Utc));
    let state = ev
        .state
        .achievements
        .iter()
        .find(|a| a.id == AchievementId::TokenMonk)
        .unwrap();
    assert_eq!((state.current, state.target), (None, None));
    assert_eq!(state.progress_label, None);

    // Max it out never earns it.
    let mut store = MemoryStore::default();
    let days = vec![
        day(first, 800, Some(5.0)),
        day(first + Days::new(1), 0, None),
    ];
    let ev = apply(&mut store, &facts(days), Mode::Max, now, false);
    assert!(monk(&ev).is_none());
}

#[test]
fn level_up_reports_levels_and_title_change() {
    let l1 = xp_for_level(1);
    let mut prev = GameData::default();
    prev.ledger.push(LedgerEntry {
        earned_at: 0,
        mode: Mode::Max,
        kind: "seed".into(),
        reference: "a".into(),
        amount: (l1 - 5) as u32,
        retroactive: false,
    });
    let f = facts(vec![day(date(2026, 10, 28), 100_000, None)]);
    let ev = evaluate(&prev, &f, Mode::Max, ms(2026, 10, 28, 12, 0), &Utc, false);
    let up = ev.level_up.expect("level up");
    assert_eq!((up.previous_level, up.level), (1, 2));
    assert!(!up.title_changed);
    assert!(!up.retroactive);
    assert_eq!(ev.state.level, 2);
    // 5 short of level 2, plus 10 for the block, 10 for the active day and
    // 10 for First Blood.
    assert_eq!(ev.state.xp_into_level, 25);

    // 2 -> 3 keeps the title inside a tier.
    let mut prev = GameData::default();
    prev.ledger.push(LedgerEntry {
        earned_at: 0,
        mode: Mode::Max,
        kind: "seed".into(),
        reference: "a".into(),
        amount: (l1 + xp_for_level(2) - 5) as u32,
        retroactive: false,
    });
    let ev = evaluate(&prev, &f, Mode::Max, ms(2026, 10, 28, 12, 0), &Utc, false);
    let up = ev.level_up.unwrap();
    assert_eq!(up.level, 3);
    assert!(!up.title_changed);
    assert_eq!(up.title, "Token Tourist");

    // 4 -> 5 crosses a tier edge.
    let mut prev = GameData::default();
    prev.ledger.push(LedgerEntry {
        earned_at: 0,
        mode: Mode::Max,
        kind: "seed".into(),
        reference: "a".into(),
        amount: ((1..5).map(xp_for_level).sum::<u64>() - 5) as u32,
        retroactive: false,
    });
    let ev = evaluate(&prev, &f, Mode::Max, ms(2026, 10, 28, 12, 0), &Utc, false);
    let up = ev.level_up.unwrap();
    assert_eq!((up.previous_level, up.level), (4, 5));
    assert!(up.title_changed);
    assert_eq!(up.title, "Prompt Regular");
}

#[test]
fn state_matches_the_curve() {
    let mut store = MemoryStore::default();
    let f = facts(vec![day(date(2026, 10, 28), 1_900_000, None)]);
    let ev = apply(&mut store, &f, Mode::Max, ms(2026, 10, 28, 12, 0), false);
    let info = level_info(ev.state.xp_total);
    assert_eq!(ev.state.level, info.level);
    assert_eq!(ev.state.xp_into_level, info.into_level);
    assert_eq!(ev.state.xp_for_level, info.for_level);
    assert!((0.0..=1.0).contains(&ev.state.progress));
    assert_eq!(ev.state.achievements.len(), 6);
    assert_eq!(ev.state.achievements[0].id, AchievementId::FirstBlood);
}

#[test]
fn boo_is_not_earnable() {
    let mut store = MemoryStore::default();
    let now = ms(2026, 11, 2, 12, 0);
    let f = Facts::default();
    for mode in [Mode::Max, Mode::Diy] {
        let ev = apply(&mut store, &f, mode, now, false);
        assert!(!unlocked(&ev).contains(&AchievementId::Boo));
        assert_eq!(ev.state.xp_total, 0);
    }
}

fn legacy_boo() -> GameData {
    let at = ms(2026, 10, 28, 12, 0);
    GameData {
        ledger: vec![LedgerEntry {
            earned_at: at,
            mode: Mode::Max,
            kind: KIND_ACHIEVEMENT.to_string(),
            reference: "boo".to_string(),
            amount: 100,
            retroactive: false,
        }],
        achievements: vec![AchievementRecord {
            id: AchievementId::Boo,
            unlocked_at: Some(at),
            retroactive: false,
            current: None,
            target: None,
            updated_at: at,
        }],
        retro_done: true,
    }
}

#[test]
fn legacy_boo_stays_unlocked_and_is_never_regranted() {
    let mut store = MemoryStore {
        data: legacy_boo(),
        ..MemoryStore::default()
    };
    let now = ms(2026, 11, 2, 12, 0);
    let ev = apply(&mut store, &Facts::default(), Mode::Max, now, false);
    assert!(ev.unlocks.is_empty() && ev.ledger.is_empty());
    assert_eq!(ev.state.xp_total, 100);
    let boo = ev
        .state
        .achievements
        .iter()
        .find(|a| a.id == AchievementId::Boo)
        .unwrap();
    assert!(boo.unlocked_at.is_some());
    assert_eq!(boo.name, "Halloween 2026");
}

#[test]
fn retroactive_levelling_never_grants_boo() {
    let mut store = MemoryStore::default();
    let first = date(2026, 10, 1);
    let days: Vec<_> = (0..10)
        .map(|i| day(first + Days::new(i), 800, Some(5.0)))
        .collect();
    let now = day_start_ms(first + Days::new(9), &Utc) + 12 * HOUR;
    let ev = apply(&mut store, &facts(days), Mode::Max, now, true);
    assert!(!unlocked(&ev).contains(&AchievementId::Boo));
    assert!(store
        .data
        .achievements
        .iter()
        .all(|r| r.id != AchievementId::Boo));
}

// --- retroactive replay == live -------------------------------------------

struct History {
    usage: Vec<(i64, u64)>,
    limits: Vec<LimitFact>,
}

/// Deterministic synthetic history of 40 days from 2026-09-01. The first
/// `calm_days` days are tiny.
fn history(calm_days: u64) -> History {
    let start = date(2026, 9, 1);
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move |modulo: u64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % modulo
    };
    let mut usage = Vec::new();
    for i in 0..40u64 {
        let base = day_start_ms(start + Days::new(i), &Utc);
        if i < calm_days {
            for _ in 0..2 {
                usage.push((base + (6 + next(14) as i64) * HOUR, 300 + next(600)));
            }
        } else if i % 5 != 4 {
            for _ in 0..(3 + next(6)) {
                usage.push((
                    base + (6 + next(16) as i64) * HOUR + next(60) as i64 * 60_000,
                    20_000 + next(130_000),
                ));
            }
            if i % 7 == 3 {
                usage.push((base + 2 * HOUR + 30 * 60_000, 4_000));
            }
        }
    }
    usage.sort();
    let d5 = day_start_ms(start + Days::new(5), &Utc);
    let d9 = day_start_ms(start + Days::new(9), &Utc);
    let limits = vec![
        limit(d5 + 2 * HOUR, 40.0, d5 + 5 * HOUR),
        limit(d5 + 3 * HOUR, 99.4, d5 + 5 * HOUR),
        limit(d5 + 4 * HOUR, 99.2, d5 + 5 * HOUR),
        limit(d9 + 8 * HOUR + 30 * 60_000, 100.0, d9 + 10 * HOUR),
    ];
    History { usage, limits }
}

/// Facts as the app layer would build them for `now`, from `from` through
/// the day of `now`. Usage after `now` does not exist yet.
fn facts_at(h: &History, now: i64, from: NaiveDate) -> Facts {
    let today = local_date(now, &Utc);
    let mut days = Vec::new();
    let mut d = from;
    while d <= today {
        let lo = day_start_ms(d, &Utc);
        let hi = lo + DAY;
        let usage: Vec<_> = h
            .usage
            .iter()
            .copied()
            .filter(|(ts, _)| *ts >= lo && *ts < hi && *ts <= now)
            .collect();
        let tokens: u64 = usage.iter().map(|(_, t)| t).sum();
        // Budget share of a day: 10,000 tokens are 1 %.
        let budget = Some(tokens as f64 / 10_000.0);
        days.push(DayFacts::from_usage(d, usage, budget, &Utc));
        d = d + Days::new(1);
    }
    Facts {
        days,
        limits: h.limits.iter().copied().filter(|l| l.at <= now).collect(),
    }
}

type LedgerView = BTreeMap<(String, String), (i64, Mode, u32)>;

fn ledger_view(data: &GameData) -> LedgerView {
    data.ledger
        .iter()
        .map(|e| {
            (
                (e.kind.clone(), e.reference.clone()),
                (e.earned_at, e.mode, e.amount),
            )
        })
        .collect()
}

fn achievement_view(data: &GameData) -> BTreeMap<&'static str, (Option<i64>, Option<u32>)> {
    data.achievements
        .iter()
        .map(|r| (r.id.as_str(), (r.unlocked_at, r.current)))
        .collect()
}

fn replay_equals_live(mode: Mode, calm_days: u64) {
    let h = history(calm_days);
    let first = date(2026, 9, 1);
    let end = ms(2026, 10, 10, 12, 0);

    let mut replay = MemoryStore::default();
    let r = run_evaluation(
        &mut replay,
        &facts_at(&h, end, first),
        mode,
        end,
        &Utc,
        true,
    )
    .unwrap();
    assert!(r.state.retro_done);
    assert!(replay.data.retro_done);
    assert!(replay.data.ledger.iter().all(|e| e.retroactive));

    // Live: an evaluation every six hours with the trailing window only.
    let mut live = MemoryStore::default();
    let mut now = ms(2026, 9, 1, 0, 0);
    while now <= end {
        let today = local_date(now, &Utc);
        let from = (today - Days::new(u64::from(STREAK_LOOKBACK_DAYS) - 1)).max(first);
        run_evaluation(&mut live, &facts_at(&h, now, from), mode, now, &Utc, false).unwrap();
        now += 6 * HOUR;
    }
    // The last live step lands exactly on `end`.
    assert_eq!((now - end) % (6 * HOUR), 0);

    assert!(!replay.data.ledger.is_empty());
    assert_eq!(ledger_view(&replay.data), ledger_view(&live.data));
    assert_eq!(achievement_view(&replay.data), achievement_view(&live.data));

    let lv = &live.load().unwrap();
    let a = evaluate(&replay.data, &Facts::default(), mode, end, &Utc, false).state;
    let b = evaluate(lv, &Facts::default(), mode, end, &Utc, false).state;
    assert_eq!(a.xp_total, b.xp_total);
    assert_eq!(a.level, b.level);
    assert_eq!(a.title, b.title);
    assert_eq!(a.xp_into_level, b.xp_into_level);
    for (x, y) in a.achievements.iter().zip(&b.achievements) {
        assert_eq!(
            (x.unlocked_at, x.current, &x.progress_label),
            (y.unlocked_at, y.current, &y.progress_label)
        );
    }

    // Replaying again changes nothing.
    let again = run_evaluation(
        &mut replay,
        &facts_at(&h, end, first),
        mode,
        end,
        &Utc,
        true,
    )
    .unwrap();
    assert!(again.ledger.is_empty() && again.unlocks.is_empty());
}

#[test]
fn replay_equals_live_for_max_it_out() {
    replay_equals_live(Mode::Max, 0);

    // Spot checks on the replay itself.
    let h = history(0);
    let end = ms(2026, 10, 10, 12, 0);
    let ev = evaluate(
        &GameData::default(),
        &facts_at(&h, end, date(2026, 9, 1)),
        Mode::Max,
        end,
        &Utc,
        true,
    );
    let ids = unlocked(&ev);
    for id in [
        AchievementId::FirstBlood,
        AchievementId::NightOwl,
        AchievementId::RateLimitRambo,
        AchievementId::SoClose,
    ] {
        assert!(ids.contains(&id), "{id:?} missing");
    }
    assert!(!ids.contains(&AchievementId::TokenMonk));
    assert!(ev.unlocks.iter().all(|u| u.retroactive));
    assert!(ev.level_up.as_ref().is_some_and(|l| l.retroactive));
    assert!(ev.state.level > 1);
}

#[test]
fn replay_equals_live_for_do_it_yourself() {
    replay_equals_live(Mode::Diy, 10);

    let h = history(10);
    let end = ms(2026, 10, 10, 12, 0);
    let ev = evaluate(
        &GameData::default(),
        &facts_at(&h, end, date(2026, 9, 1)),
        Mode::Diy,
        end,
        &Utc,
        true,
    );
    assert!(unlocked(&ev).contains(&AchievementId::TokenMonk));
    assert!(!unlocked(&ev).contains(&AchievementId::NightOwl));
}
