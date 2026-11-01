//! Builds the game's [`Facts`] from the database.
//!
//! The game module is pure; this is the bridge from stored messages and
//! limit snapshots to `DayFacts` and `LimitFact`. All reads go through the
//! database helpers; no SQL lives here.

use std::collections::{BTreeMap, HashMap};

use chrono::{NaiveDate, TimeZone};

use crate::aggregate::{
    add_days, day_start, days_left, history_budget, local_date, weekly_budget, HISTORY_DAYS,
    WEEK_MS,
};
use crate::contracts::{EpochMs, Mode};
use crate::db::{Db, DbResult};
use crate::game::constants::STREAK_LOOKBACK_DAYS;
use crate::game::{DayFacts, Facts, LimitFact};

/// Live evaluations look at readings of this many hours.
const LIVE_LIMIT_HOURS: i64 = 12;

/// Per-day budget percentages of finished days, kept between evaluations.
pub type BudgetCache = HashMap<NaiveDate, Option<f64>>;

/// What range an evaluation covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Replay of the whole history (first start after onboarding).
    Replay,
    /// After new data: at least the streak lookback, extended back to
    /// `touched` (the oldest day with new usage) so every touched day is
    /// complete.
    Live { touched: Option<EpochMs> },
}

/// First day of the evaluation range, or `None` if a replay has no usage.
fn first_day<Tz: TimeZone>(
    db: &Db,
    scope: Scope,
    today: NaiveDate,
    tz: &Tz,
) -> DbResult<Option<NaiveDate>> {
    match scope {
        Scope::Replay => Ok(db.first_message_ts()?.map(|ts| local_date(tz, ts))),
        Scope::Live { touched } => {
            let floor = add_days(today, -(i64::from(STREAK_LOOKBACK_DAYS) - 1));
            Ok(Some(match touched {
                Some(ts) => floor.min(local_date(tz, ts)),
                None => floor,
            }))
        }
    }
}

/// Stored 5-hour readings with `fetched_at >= from`, oldest first.
fn limit_facts(db: &Db, from: EpochMs) -> DbResult<Vec<LimitFact>> {
    Ok(db
        .five_hour_snapshots_since(from)?
        .into_iter()
        .map(|s| LimitFact {
            at: s.at,
            five_hour_pct: Some(s.pct),
            five_hour_resets_at: s.resets_at,
        })
        .collect())
}

/// Budget used (percent) by a finished `day`, on the basis that was
/// available then: the weekly limit (last stored weekly reading of that day,
/// if its window was still running) else the token history of the 7 days
/// before. `None` without either.
fn finished_day_budget<Tz: TimeZone>(
    db: &Db,
    day: NaiveDate,
    tokens: u64,
    tz: &Tz,
) -> DbResult<Option<f64>> {
    let start = day_start(tz, day);
    let end = day_start(tz, add_days(day, 1));
    let last = end - 1;

    let weekly = db.last_weekly_before(end)?;
    if let Some((pct, Some(resets_at))) = weekly.filter(|(_, r)| r.is_some_and(|r| r > last)) {
        let window_start = resets_at.saturating_sub(WEEK_MS);
        let w0 = if window_start > start {
            0.0
        } else {
            db.weekly_baseline(start, Some(window_start))?
                .unwrap_or(pct)
        };
        let left = days_left(day, local_date(tz, resets_at));
        return Ok(Some(weekly_budget(pct, w0, left, resets_at).used_percent));
    }

    let from = day_start(tz, add_days(day, -HISTORY_DAYS));
    let previous: Vec<u64> = db
        .tokens_by_local_day(tz, from, start)?
        .iter()
        .map(|d| d.totals.headline())
        .collect();
    Ok(history_budget(&previous, tokens).map(|b| b.used_percent))
}

/// Facts for one evaluation.
///
/// * `today_budget`: the live snapshot's budget percentage for today
///   (Do it yourself only; pass `None` otherwise).
/// * `cache`: budgets of finished days; filled here.
pub fn build_facts<Tz: TimeZone>(
    db: &Db,
    mode: Mode,
    scope: Scope,
    now: EpochMs,
    today_budget: Option<f64>,
    cache: &mut BudgetCache,
    tz: &Tz,
) -> DbResult<Facts> {
    let today = local_date(tz, now);
    let Some(first) = first_day(db, scope, today, tz)? else {
        return Ok(Facts::default());
    };
    let first = first.min(today);

    // A finished day that gained usage later (startup backlog, late flush)
    // has a stale budget, and so has every day after it (the history basis
    // reaches back 7 days). Drop those before they are read.
    if let Scope::Live {
        touched: Some(touched),
    } = scope
    {
        let from = local_date(tz, touched);
        cache.retain(|day, _| *day < from);
    }

    let range_start = day_start(tz, first);
    let range_end = day_start(tz, add_days(today, 1));
    let mut usage: BTreeMap<NaiveDate, Vec<(EpochMs, u64)>> = BTreeMap::new();
    for (ts, tokens) in db.usage_between(range_start, range_end)? {
        usage
            .entry(local_date(tz, ts))
            .or_default()
            .push((ts, tokens));
    }

    let mut days = Vec::new();
    let mut day = first;
    while day <= today {
        let rows = usage.remove(&day).unwrap_or_default();
        let tokens: u64 = rows.iter().map(|(_, t)| t).sum();
        let budget = if mode != Mode::Diy {
            None
        } else if day == today {
            today_budget
        } else if let Some(cached) = cache.get(&day) {
            *cached
        } else {
            let b = finished_day_budget(db, day, tokens, tz)?;
            cache.insert(day, b);
            b
        };
        days.push(DayFacts::from_usage(day, rows, budget, tz));
        day = add_days(day, 1);
    }

    let limits_from = match scope {
        Scope::Replay => 0,
        Scope::Live { .. } => now - LIVE_LIMIT_HOURS * 3_600_000,
    };
    Ok(Facts {
        days,
        limits: limit_facts(db, limits_from)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::UsageRecord;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn msg(id: &str, ts: EpochMs, out: u64) -> UsageRecord {
        UsageRecord {
            message_id: id.into(),
            request_id: None,
            session_id: "s".into(),
            agent_id: None,
            is_sidechain: false,
            ts,
            model: "m".into(),
            project_path: "/p".into(),
            input_tokens: 10,
            output_tokens: out,
            cache_creation_tokens: 5,
            cache_read_tokens: 9_999,
        }
    }

    fn ms(y: i32, m: u32, d: u32, h: u32) -> EpochMs {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis()
    }

    #[test]
    fn replay_covers_first_usage_day_to_today_with_idle_days() {
        let mut db = Db::open_in_memory().unwrap();
        db.upsert_messages(&[
            msg("a", ms(2026, 10, 25, 3), 100),
            msg("b", ms(2026, 10, 25, 14), 200),
            msg("c", ms(2026, 10, 27, 9), 50),
        ])
        .unwrap();
        let now = ms(2026, 10, 28, 12);
        let mut cache = BudgetCache::new();
        let facts =
            build_facts(&db, Mode::Max, Scope::Replay, now, None, &mut cache, &tz()).unwrap();
        let days: Vec<_> = facts
            .days
            .iter()
            .map(|d| (d.day.to_string(), d.tokens))
            .collect();
        assert_eq!(
            days,
            [
                ("2026-10-25".into(), 115 + 215),
                ("2026-10-26".into(), 0),
                ("2026-10-27".into(), 65),
                ("2026-10-28".into(), 0),
            ]
        );
        // Night Owl window is 02:00 to 04:00.
        assert_eq!(facts.days[0].night_at, Some(ms(2026, 10, 25, 3)));
        assert_eq!(facts.days[0].first_at, Some(ms(2026, 10, 25, 3)));
        assert!(cache.is_empty(), "Max it out needs no budgets");
    }

    #[test]
    fn live_covers_the_lookback_and_extends_to_touched_days() {
        let db = Db::open_in_memory().unwrap();
        let now = ms(2026, 10, 28, 12);
        let mut cache = BudgetCache::new();
        let live = |touched| Scope::Live { touched };
        let facts = build_facts(&db, Mode::Max, live(None), now, None, &mut cache, &tz()).unwrap();
        assert_eq!(facts.days.len(), STREAK_LOOKBACK_DAYS as usize);
        assert_eq!(facts.days.last().unwrap().day.to_string(), "2026-10-28");
        assert_eq!(facts.days[0].day.to_string(), "2026-10-15");
        // A record from 20 days ago extends the range.
        let old = ms(2026, 10, 8, 10);
        let facts = build_facts(
            &db,
            Mode::Max,
            live(Some(old)),
            now,
            None,
            &mut cache,
            &tz(),
        )
        .unwrap();
        assert_eq!(facts.days[0].day.to_string(), "2026-10-08");
        assert_eq!(facts.days.len(), 21);
    }

    #[test]
    fn late_usage_evicts_cached_budgets_from_the_touched_day_on() {
        let mut db = Db::open_in_memory().unwrap();
        db.upsert_messages(&[
            msg("a", ms(2026, 10, 24, 10), 985),
            msg("b", ms(2026, 10, 25, 10), 985),
            msg("c", ms(2026, 10, 26, 10), 185),
        ])
        .unwrap();
        let now = ms(2026, 10, 27, 12);
        let mut cache = BudgetCache::new();
        let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
        let live = |touched| Scope::Live { touched };
        build_facts(&db, Mode::Diy, live(None), now, None, &mut cache, &tz()).unwrap();
        let before = cache[&day(26)].unwrap();
        assert!(cache.contains_key(&day(25)));

        // The 26th gains a lot of usage afterwards.
        db.upsert_messages(&[msg("late", ms(2026, 10, 26, 18), 4_000)])
            .unwrap();
        let facts = build_facts(
            &db,
            Mode::Diy,
            live(Some(ms(2026, 10, 26, 18))),
            now,
            None,
            &mut cache,
            &tz(),
        )
        .unwrap();
        let after = cache[&day(26)].unwrap();
        assert!(after > before, "{after} should exceed {before}");
        let fact = facts.days.iter().find(|d| d.day == day(26)).unwrap();
        assert_eq!(fact.budget_used_percent, Some(after));
        // Earlier days keep their cached value.
        assert!(cache.contains_key(&day(25)));
    }

    #[test]
    fn diy_budgets_use_history_for_finished_days_and_the_snapshot_for_today() {
        let mut db = Db::open_in_memory().unwrap();
        // Two active days, 1000 tokens each (headline 10 + 985 + 5), then a
        // light day at 20 % of that average.
        db.upsert_messages(&[
            msg("a", ms(2026, 10, 24, 10), 985),
            msg("b", ms(2026, 10, 25, 10), 985),
            msg("c", ms(2026, 10, 26, 10), 185),
        ])
        .unwrap();
        let now = ms(2026, 10, 27, 12);
        let mut cache = BudgetCache::new();
        let facts = build_facts(
            &db,
            Mode::Diy,
            Scope::Live { touched: None },
            now,
            Some(42.0),
            &mut cache,
            &tz(),
        )
        .unwrap();
        let pct = |d: &str| {
            facts
                .days
                .iter()
                .find(|x| x.day.to_string() == d)
                .and_then(|x| x.budget_used_percent)
        };
        assert_eq!(pct("2026-10-24"), None, "no previous active day");
        assert!((pct("2026-10-26").unwrap() - 20.0).abs() < 1e-6);
        assert_eq!(pct("2026-10-27"), Some(42.0));
        assert_eq!(cache.get(&facts.days[0].day), Some(&None));
        assert!(!cache.contains_key(&facts.days.last().unwrap().day));
    }

    #[test]
    fn limit_facts_are_read_from_snapshots() {
        let db = Db::open_in_memory().unwrap();
        for (at, pct) in [(1_000, 50.0), (2_000, 100.0)] {
            db.insert_limit_snapshot(
                "statusline",
                &crate::contracts::LimitReading {
                    five_hour: Some(crate::contracts::LimitWindow {
                        utilization: pct,
                        resets_at: Some(at + 10),
                    }),
                    weekly: None,
                    fetched_at: at,
                },
            )
            .unwrap();
        }
        assert_eq!(limit_facts(&db, 0).unwrap().len(), 2);
        let late = limit_facts(&db, 1_500).unwrap();
        assert_eq!(late.len(), 1);
        assert_eq!(late[0].five_hour_pct, Some(100.0));
    }
}
