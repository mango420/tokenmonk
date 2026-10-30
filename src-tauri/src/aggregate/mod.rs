//! Builds the `DashboardSnapshot` from the database and the latest limit
//! reading (docs/ARCHITECTURE.md, "Aggregates").
//!
//! [`snapshot`] reads; it never writes. Calendar boundaries use the given
//! time zone, storage is UTC epoch ms. The window lengths and thresholds
//! below are the only place these numbers live.

mod budget;
mod limit_state;
mod timeutil;

use chrono::TimeZone;

use crate::contracts::dashboard::{
    BurnRate, DailyBudget, DashboardSnapshot, DataStatus, SessionStats, TokenBreakdown,
};
use crate::contracts::settings::Mode;
use crate::contracts::EpochMs;
use crate::db::{Db, DbResult, TokenTotals};

pub use budget::{days_left, history_budget, weekly_budget};
pub use limit_state::{limit_eta_minutes, LimitState};
pub use timeutil::{add_days, day_start, local_date};

/// A session is current while its newest message is younger than this.
pub const SESSION_IDLE_MS: i64 = 60 * 60_000;
/// Burn rate window in minutes (`BurnRate::window_minutes`).
pub const BURN_WINDOW_MINUTES: u32 = 15;
/// A limit reading older than this is `stale`.
pub const LIMIT_STALE_MS: i64 = 15 * 60_000;
/// Length of the 5-hour limit window.
pub const FIVE_HOUR_MS: i64 = 5 * 60 * 60_000;
/// Length of the weekly limit window.
pub const WEEK_MS: i64 = 7 * 24 * 60 * 60_000;
/// Previous local days that feed the token-history budget.
pub const HISTORY_DAYS: i64 = 7;

fn breakdown(t: &TokenTotals) -> TokenBreakdown {
    TokenBreakdown {
        input: t.input,
        output: t.output,
        cache_creation: t.cache_creation,
        cache_read: t.cache_read,
        total: t.headline(),
    }
}

/// Last path component of a working directory; handles `/` and `\`.
pub fn project_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}

fn current_session(db: &Db, now: EpochMs) -> DbResult<Option<SessionStats>> {
    let Some(latest) = db.latest_message()? else {
        return Ok(None);
    };
    if now.saturating_sub(latest.ts) >= SESSION_IDLE_MS {
        return Ok(None);
    }
    let Some(s) = db.session_summary(&latest.session_id)? else {
        return Ok(None);
    };
    let project = db
        .session_project_path(&s.session_id)?
        .map_or_else(|| "unknown".to_owned(), |p| project_name(&p));
    Ok(Some(SessionStats {
        session_id: s.session_id,
        project_name: project,
        started_at: s.first_ts,
        last_activity_at: s.last_ts,
        tokens: breakdown(&s.totals),
    }))
}

/// Computes the snapshot for time `now`.
///
/// * `limits`: latest limit knowledge (see [`LimitState`]).
/// * `mode`: the chosen play mode. The token-history budget is the fallback
///   of Do it yourself only, so it is skipped in Max it out.
/// * `data`: import state from the ingest worker; `last_message_at` is
///   filled in here from the database.
pub fn snapshot<Tz: TimeZone>(
    db: &Db,
    limits: &LimitState,
    mode: Mode,
    mut data: DataStatus,
    now: EpochMs,
    tz: &Tz,
) -> DbResult<DashboardSnapshot> {
    let today_date = local_date(tz, now);
    let today_start = day_start(tz, today_date);
    let tomorrow_start = day_start(tz, add_days(today_date, 1));
    let today_totals = db.tokens_between(today_start, tomorrow_start)?;

    let burn_window_ms = i64::from(BURN_WINDOW_MINUTES) * 60_000;
    let burning = db.tokens_trailing(now, burn_window_ms)?;
    let tokens_per_minute = burning.headline() as f64 / f64::from(BURN_WINDOW_MINUTES);

    let view = limits.view(now);

    let eta = match view.five_hour.as_ref() {
        Some(w) if w.utilization >= 100.0 => limit_eta_minutes(w, now, tokens_per_minute, 0),
        Some(w) => match w.resets_at {
            Some(resets_at) => {
                let start = resets_at.saturating_sub(FIVE_HOUR_MS);
                let in_window = db.tokens_between(start, now.saturating_add(1))?.headline();
                limit_eta_minutes(w, now, tokens_per_minute, in_window)
            }
            None => None,
        },
        None => None,
    };

    let daily_budget: Option<DailyBudget> = match view.weekly.as_ref() {
        Some(w) => budget::weekly_from_db(db, w, now, tz)?,
        None => None,
    };
    let daily_budget = match daily_budget {
        Some(b) => Some(b),
        None if mode != Mode::Max => budget::history_from_db(db, today_totals.headline(), now, tz)?,
        None => None,
    };

    data.last_message_at = db.latest_message()?.map(|m| m.ts);

    Ok(DashboardSnapshot {
        generated_at: now,
        today: breakdown(&today_totals),
        session: current_session(db, now)?,
        burn_rate: BurnRate {
            tokens_per_minute,
            window_minutes: BURN_WINDOW_MINUTES,
            limit_eta_minutes: eta,
        },
        limits: view,
        daily_budget,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::dashboard::{BudgetBasis, DataState};
    use crate::contracts::limits::{LimitReading, LimitReason, LimitStatus, LimitWindow};
    use crate::limits::LimitPoll;
    use crate::logs::UsageRecord;
    use chrono::FixedOffset;

    const MIN: i64 = 60_000;
    const HOUR: i64 = 60 * MIN;
    const DAY: i64 = 24 * HOUR;
    // 2026-10-28T00:00:00Z
    const DAY0: i64 = 1_793_145_600_000;

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn data() -> DataStatus {
        DataStatus {
            state: DataState::Ready,
            files_total: 0,
            files_done: 0,
            last_message_at: None,
            detail: None,
        }
    }

    fn rec(
        id: &str,
        session: &str,
        ts: i64,
        input: u64,
        out: u64,
        cc: u64,
        cr: u64,
    ) -> UsageRecord {
        UsageRecord {
            message_id: id.into(),
            request_id: None,
            session_id: session.into(),
            agent_id: None,
            is_sidechain: false,
            ts,
            model: "m".into(),
            project_path: "C:\\work\\demo-app".into(),
            input_tokens: input,
            output_tokens: out,
            cache_creation_tokens: cc,
            cache_read_tokens: cr,
        }
    }

    fn snap(db: &Db, limits: &LimitState, mode: Mode, now: i64) -> DashboardSnapshot {
        snapshot(db, limits, mode, data(), now, &tz()).unwrap()
    }

    #[test]
    fn empty_database_gives_zeroes_and_no_session() {
        let db = Db::open_in_memory().unwrap();
        let s = snap(&db, &LimitState::default(), Mode::Max, DAY0 + 12 * HOUR);
        assert_eq!(s.today, TokenBreakdown::default());
        assert!(s.session.is_none());
        assert_eq!(s.burn_rate.tokens_per_minute, 0.0);
        assert_eq!(s.burn_rate.window_minutes, 15);
        assert_eq!(s.burn_rate.limit_eta_minutes, None);
        assert_eq!(s.limits.status, LimitStatus::Unavailable);
        assert!(s.daily_budget.is_none());
        assert_eq!(s.data.last_message_at, None);
    }

    #[test]
    fn today_excludes_cache_reads_from_total_and_other_days() {
        let mut db = Db::open_in_memory().unwrap();
        let now = DAY0 + 12 * HOUR;
        db.upsert_messages(&[
            rec("a", "s1", DAY0 + HOUR, 10, 20, 30, 5000),
            rec("b", "s1", DAY0 + 2 * HOUR, 1, 2, 3, 4),
            rec("old", "s1", DAY0 - HOUR, 100, 100, 100, 100),
        ])
        .unwrap();
        let s = snap(&db, &LimitState::default(), Mode::Max, now);
        assert_eq!(s.today.input, 11);
        assert_eq!(s.today.output, 22);
        assert_eq!(s.today.cache_creation, 33);
        assert_eq!(s.today.cache_read, 5004);
        assert_eq!(s.today.total, 66);
        assert_eq!(s.data.last_message_at, Some(DAY0 + 2 * HOUR));
    }

    #[test]
    fn day_rollover_uses_the_local_zone() {
        let mut db = Db::open_in_memory().unwrap();
        // 23:30Z on the 28th is already the 29th at +02:00.
        db.upsert_messages(&[rec("a", "s", DAY0 + 23 * HOUR + 30 * MIN, 5, 5, 0, 0)])
            .unwrap();
        let east = FixedOffset::east_opt(2 * 3600).unwrap();
        let now = DAY0 + 23 * HOUR + 40 * MIN;
        let local = snapshot(&db, &LimitState::default(), Mode::Max, data(), now, &east).unwrap();
        assert_eq!(local.today.total, 10);
        // One hour later in UTC terms the 29th starts there too; the 28th totals vanish.
        let next_utc_day = now + 30 * MIN;
        let utc = snapshot(
            &db,
            &LimitState::default(),
            Mode::Max,
            data(),
            next_utc_day,
            &tz(),
        )
        .unwrap();
        assert_eq!(utc.today.total, 0);
    }

    #[test]
    fn session_follows_latest_message_with_idle_cut_off() {
        let mut db = Db::open_in_memory().unwrap();
        let t = DAY0 + 10 * HOUR;
        db.upsert_messages(&[
            rec("a", "s-old", t - 5 * HOUR, 1, 1, 0, 0),
            rec("b", "s-cur", t - 3 * HOUR, 10, 10, 0, 0),
            rec("c", "s-cur", t - 30 * MIN, 20, 20, 5, 9),
        ])
        .unwrap();
        let s = snap(&db, &LimitState::default(), Mode::Max, t);
        let session = s.session.unwrap();
        assert_eq!(session.session_id, "s-cur");
        assert_eq!(session.project_name, "demo-app");
        assert_eq!(session.started_at, t - 3 * HOUR);
        assert_eq!(session.last_activity_at, t - 30 * MIN);
        // Sums the whole session, not just the last hour.
        assert_eq!(session.tokens.total, 65);
        assert_eq!(session.tokens.cache_read, 9);

        // Exactly 60 minutes after the last message the session is idle.
        let idle = snap(&db, &LimitState::default(), Mode::Max, t - 30 * MIN + HOUR);
        assert!(idle.session.is_none());
        let still = snap(
            &db,
            &LimitState::default(),
            Mode::Max,
            t - 30 * MIN + HOUR - 1,
        );
        assert!(still.session.is_some());
    }

    #[test]
    fn burn_rate_uses_the_trailing_15_minutes_and_decays() {
        let mut db = Db::open_in_memory().unwrap();
        let now = DAY0 + 10 * HOUR;
        db.upsert_messages(&[
            rec("a", "s", now - 20 * MIN, 1000, 0, 0, 0),
            rec("b", "s", now - 10 * MIN, 100, 100, 100, 99_999),
        ])
        .unwrap();
        let s = snap(&db, &LimitState::default(), Mode::Max, now);
        assert!((s.burn_rate.tokens_per_minute - 20.0).abs() < 1e-9);
        let later = snap(&db, &LimitState::default(), Mode::Max, now + 10 * MIN + 1);
        assert_eq!(later.burn_rate.tokens_per_minute, 0.0);
    }

    fn limits_with(reading: LimitReading) -> LimitState {
        let mut st = LimitState::default();
        st.apply(&LimitPoll {
            source: "test",
            result: Ok(reading),
        });
        st
    }

    #[test]
    fn eta_from_five_hour_window_tokens() {
        let mut db = Db::open_in_memory().unwrap();
        let now = DAY0 + 10 * HOUR;
        let window_start = now - 2 * HOUR;
        // 4000 headline tokens in the window, 1500 of them in the last 15 minutes
        // (the 15 minute window is inclusive of its start).
        db.upsert_messages(&[
            rec("a", "s", window_start + MIN, 2500, 0, 0, 0),
            rec("b", "s", now - 5 * MIN, 1500, 0, 0, 0),
            rec("before", "s", window_start - MIN, 9000, 0, 0, 0),
        ])
        .unwrap();
        let limits = limits_with(LimitReading {
            five_hour: Some(LimitWindow {
                utilization: 40.0,
                resets_at: Some(window_start + FIVE_HOUR_MS),
            }),
            weekly: None,
            fetched_at: now - MIN,
        });
        let s = snap(&db, &limits, Mode::Max, now);
        // burn = 1500/15 = 100 tokens/min; 0.01 %/token -> 1 %/min; 60 % left.
        assert!((s.burn_rate.tokens_per_minute - 100.0).abs() < 1e-9);
        assert_eq!(s.burn_rate.limit_eta_minutes, Some(60));
        assert_eq!(s.limits.status, LimitStatus::Ok);
    }

    #[test]
    fn eta_is_none_without_reading_and_zero_at_limit() {
        let mut db = Db::open_in_memory().unwrap();
        let now = DAY0 + 10 * HOUR;
        db.upsert_messages(&[rec("a", "s", now - MIN, 100, 0, 0, 0)])
            .unwrap();
        let s = snap(&db, &LimitState::default(), Mode::Max, now);
        assert_eq!(s.burn_rate.limit_eta_minutes, None);
        let full = limits_with(LimitReading {
            five_hour: Some(LimitWindow {
                utilization: 100.0,
                resets_at: Some(now + HOUR),
            }),
            weekly: None,
            fetched_at: now,
        });
        assert_eq!(
            snap(&db, &full, Mode::Max, now).burn_rate.limit_eta_minutes,
            Some(0)
        );
    }

    #[test]
    fn weekly_budget_uses_the_baseline_from_the_start_of_the_day() {
        let db = Db::open_in_memory().unwrap();
        let now = DAY0 + 12 * HOUR;
        // Reset on the 31st at 06:00Z: 4 local days (28, 29, 30, 31).
        let resets_at = DAY0 + 3 * DAY + 6 * HOUR;
        let earlier = LimitReading {
            five_hour: None,
            weekly: Some(LimitWindow {
                utilization: 30.0,
                resets_at: Some(resets_at),
            }),
            fetched_at: DAY0 - HOUR,
        };
        db.insert_limit_snapshot("test", &earlier).unwrap();
        let current = LimitReading {
            weekly: Some(LimitWindow {
                utilization: 37.0,
                resets_at: Some(resets_at),
            }),
            fetched_at: now - MIN,
            ..earlier
        };
        db.insert_limit_snapshot("test", &current).unwrap();
        let limits = limits_with(current);
        let b = snap(&db, &limits, Mode::Diy, now).daily_budget.unwrap();
        assert_eq!(b.basis, BudgetBasis::WeeklyLimit);
        assert_eq!(b.days_left, 4);
        assert!((b.allowance - 17.5).abs() < 1e-9);
        assert!((b.used_today - 7.0).abs() < 1e-9);
        assert!((b.used_percent - 40.0).abs() < 1e-9);
        assert_eq!(b.resets_at, Some(resets_at));
    }

    #[test]
    fn weekly_window_restarting_today_starts_at_zero() {
        let db = Db::open_in_memory().unwrap();
        let now = DAY0 + 12 * HOUR;
        let resets_at = now + 7 * DAY - 6 * HOUR; // window started 6 h ago
        db.insert_limit_snapshot(
            "test",
            &LimitReading {
                five_hour: None,
                weekly: Some(LimitWindow {
                    utilization: 90.0,
                    resets_at: Some(DAY0 + 6 * HOUR),
                }),
                fetched_at: DAY0 - HOUR,
            },
        )
        .unwrap();
        let current = LimitReading {
            five_hour: None,
            weekly: Some(LimitWindow {
                utilization: 4.0,
                resets_at: Some(resets_at),
            }),
            fetched_at: now,
        };
        let b = snap(&db, &limits_with(current), Mode::Diy, now)
            .daily_budget
            .unwrap();
        assert!((b.used_today - 4.0).abs() < 1e-9);
        assert!((b.allowance - 100.0 / f64::from(b.days_left)).abs() < 1e-9);
    }

    #[test]
    fn token_history_fallback_for_diy_only() {
        let mut db = Db::open_in_memory().unwrap();
        let now = DAY0 + 12 * HOUR;
        db.upsert_messages(&[
            rec("d1", "s", DAY0 - DAY + HOUR, 1000, 0, 0, 0),
            rec("d3", "s", DAY0 - 3 * DAY + HOUR, 3000, 0, 0, 0),
            rec("far", "s", DAY0 - 8 * DAY + HOUR, 999_999, 0, 0, 0),
            rec("today", "s", DAY0 + HOUR, 500, 0, 0, 0),
        ])
        .unwrap();
        let none = LimitState::default();
        let b = snap(&db, &none, Mode::Diy, now).daily_budget.unwrap();
        assert_eq!(b.basis, BudgetBasis::TokenHistory);
        assert_eq!(b.allowance, 2000.0);
        assert_eq!(b.used_today, 500.0);
        assert_eq!(b.used_percent, 25.0);
        assert_eq!((b.days_left, b.resets_at), (1, None));
        assert!(snap(&db, &none, Mode::Max, now).daily_budget.is_none());
    }

    #[test]
    fn no_history_means_no_budget() {
        let mut db = Db::open_in_memory().unwrap();
        db.upsert_messages(&[rec("t", "s", DAY0 + HOUR, 500, 0, 0, 0)])
            .unwrap();
        let s = snap(&db, &LimitState::default(), Mode::Diy, DAY0 + 2 * HOUR);
        assert!(s.daily_budget.is_none());
    }

    #[test]
    fn stale_limits_keep_values() {
        let db = Db::open_in_memory().unwrap();
        let now = DAY0 + 12 * HOUR;
        let limits = limits_with(LimitReading {
            five_hour: Some(LimitWindow {
                utilization: 12.0,
                resets_at: Some(now + HOUR),
            }),
            weekly: None,
            fetched_at: now - 16 * MIN,
        });
        let s = snap(&db, &limits, Mode::Max, now);
        assert_eq!(s.limits.status, LimitStatus::Stale);
        assert_eq!(s.limits.reason, Some(LimitReason::Other));
        assert_eq!(s.limits.five_hour.unwrap().utilization, 12.0);
    }

    #[test]
    fn project_name_handles_both_separators() {
        assert_eq!(project_name("/work/demo-app"), "demo-app");
        assert_eq!(project_name("C:\\work\\demo-app\\"), "demo-app");
        assert_eq!(project_name(""), "unknown");
        assert_eq!(project_name("/"), "unknown");
    }
}
