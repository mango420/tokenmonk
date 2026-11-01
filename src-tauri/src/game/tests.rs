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

#[test]
fn daily_cap_limits_blocks() {
    let f = facts(vec![day(date(2026, 10, 28), 50_000_000, None)]);
    let ev = evaluate(
        &GameData::default(),
        &f,
        Mode::Max,
        ms(2026, 10, 29, 1, 0),
        &Utc,
        false,
    );
    let blocks = ev.ledger.iter().filter(|e| e.kind == KIND_TOKENS).count();
    assert_eq!(blocks as u64, MAX_BLOCKS_PER_DAY);
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
    // 01:47 UTC is 02:47 local.
    let ts = ms(2026, 10, 28, 1, 47);
    assert!(in_night_window(ts, &tz));
    assert!(!in_night_window(ts, &Utc));
    let usage = [(ts, 1_000u64), (ms(2026, 10, 28, 9, 0), 500)];
    let f = facts(vec![DayFacts::from_usage(d, usage, None, &tz)]);

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
    assert_eq!(owl.detail.as_deref(), Some("Tokens burned at 02:47"));
    assert_eq!(owl.xp, 50);
    assert_eq!(owl.unlocked_at, ts);
    assert!(!owl.retroactive);
    assert_eq!(ev.state.xp_total, 25 + 50);
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
    assert_eq!(amounts["2026-10-26"], 77);
    assert_eq!(amounts["2026-10-27"], DIY_UNMEASURED_DAY_XP);
    assert_eq!(ev.state.daily_xp_preview, Some(77));

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
fn token_monk_counts_a_streak_and_unlocks_on_the_seventh_day() {
    let mut store = MemoryStore::default();
    let first = date(2026, 10, 1);
    let calm = |i: u64| day(first + Days::new(i), 800, Some(5.0));

    // Day 5 of the streak: today is day index 4, four finished days.
    let days: Vec<_> = (0..5).map(calm).collect();
    let now = day_start_ms(first + Days::new(4), &Utc) + 12 * HOUR;
    let ev = apply(&mut store, &facts(days), Mode::Diy, now, false);
    let monk = ev
        .state
        .achievements
        .iter()
        .find(|a| a.id == AchievementId::TokenMonk)
        .unwrap();
    assert_eq!((monk.current, monk.target), (Some(4), Some(7)));
    assert_eq!(
        monk.progress_label.as_deref(),
        Some("4 of 7 days under 20 %")
    );
    assert!(monk.unlocked_at.is_none());
    assert!(!ev.unlocks.iter().any(|u| u.id == AchievementId::TokenMonk));
    assert!(
        ev.achievements
            .iter()
            .any(|r| r.id == AchievementId::TokenMonk && r.current == Some(4)),
        "progress row is written"
    );

    // A busy day breaks the run.
    let mut days: Vec<_> = (0..5).map(calm).collect();
    days.push(day(first + Days::new(5), 900_000, Some(80.0)));
    days.push(calm(6));
    let now = day_start_ms(first + Days::new(6), &Utc) + HOUR;
    let ev = apply(&mut store, &facts(days), Mode::Diy, now, false);
    let monk = ev
        .state
        .achievements
        .iter()
        .find(|a| a.id == AchievementId::TokenMonk)
        .unwrap();
    assert_eq!(monk.current, Some(0));

    // Seven calm finished days in a row.
    let days: Vec<_> = (0..9).map(calm).collect();
    let now = day_start_ms(first + Days::new(8), &Utc) + HOUR;
    let mut fresh = MemoryStore::default();
    let ev = apply(&mut fresh, &facts(days), Mode::Diy, now, false);
    let unlock = ev
        .unlocks
        .iter()
        .find(|u| u.id == AchievementId::TokenMonk)
        .expect("token monk");
    assert_eq!(unlock.xp, 150);
    assert_eq!(unlock.unlocked_at, day_start_ms(first + Days::new(7), &Utc));
    let monk = ev
        .state
        .achievements
        .iter()
        .find(|a| a.id == AchievementId::TokenMonk)
        .unwrap();
    assert_eq!(monk.current, Some(7));
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
    // 5 short of level 2, plus 10 for the block and 25 for First Blood.
    assert_eq!(ev.state.xp_into_level, 30);

    // 2 -> 3 changes the title ("Token Sprout" -> "Token Page").
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
    assert!(up.title_changed);
    assert_eq!(up.title, "Token Page");
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
    let f = Facts {
        boo_at: Some(now),
        ..Facts::default()
    };
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
    let mut store = MemoryStore { data: legacy_boo() };
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
        boo_at: None,
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
