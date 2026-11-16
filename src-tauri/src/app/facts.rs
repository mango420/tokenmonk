//! Builds the game's [`Facts`] from the database.
//!
//! The game module is pure; this is the bridge from stored messages and
//! limit snapshots to `DayFacts` and `LimitFact`. All reads go through the
//! database helpers; no SQL lives here.

use std::collections::BTreeMap;

use chrono::{NaiveDate, TimeZone};

use crate::aggregate::{add_days, day_start, local_date};
use crate::contracts::EpochMs;
use crate::db::{Db, DbResult};
use crate::game::constants::STREAK_LOOKBACK_DAYS;
use crate::game::{DayFacts, Facts, LimitFact};

/// Live evaluations look at readings of this many hours.
const LIVE_LIMIT_HOURS: i64 = 12;

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

/// Facts for one evaluation.
pub fn build_facts<Tz: TimeZone>(db: &Db, scope: Scope, now: EpochMs, tz: &Tz) -> DbResult<Facts> {
    let today = local_date(tz, now);
    let Some(first) = first_day(db, scope, today, tz)? else {
        return Ok(Facts::default());
    };
    let first = first.min(today);

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
        days.push(DayFacts::from_usage(day, rows, tz));
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
        let facts = build_facts(&db, Scope::Replay, now, &tz()).unwrap();
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
    }

    #[test]
    fn live_covers_the_lookback_and_extends_to_touched_days() {
        let db = Db::open_in_memory().unwrap();
        let now = ms(2026, 10, 28, 12);
        let live = |touched| Scope::Live { touched };
        let facts = build_facts(&db, live(None), now, &tz()).unwrap();
        assert_eq!(facts.days.len(), STREAK_LOOKBACK_DAYS as usize);
        assert_eq!(facts.days.last().unwrap().day.to_string(), "2026-10-28");
        assert_eq!(facts.days[0].day.to_string(), "2026-10-15");
        // A record from 20 days ago extends the range.
        let old = ms(2026, 10, 8, 10);
        let facts = build_facts(&db, live(Some(old)), now, &tz()).unwrap();
        assert_eq!(facts.days[0].day.to_string(), "2026-10-08");
        assert_eq!(facts.days.len(), 21);
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
