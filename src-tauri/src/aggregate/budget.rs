//! Daily budget (docs/ARCHITECTURE.md, "Daily budget").

use chrono::{NaiveDate, TimeZone};

use crate::contracts::dashboard::{BudgetBasis, DailyBudget};
use crate::contracts::EpochMs;
use crate::db::{Db, DbResult};

use super::timeutil::{add_days, day_start, local_date};
use super::{HISTORY_DAYS, WEEK_MS};

fn percent(used: f64, allowance: f64) -> f64 {
    if allowance > 0.0 {
        used / allowance * 100.0
    } else {
        // Nothing left to spend: report "at the limit" rather than infinity.
        100.0
    }
}

/// Local calendar days from `today` to the reset day, both included; at least 1.
pub fn days_left(today: NaiveDate, resets_on: NaiveDate) -> u32 {
    let days = (resets_on - today).num_days() + 1;
    u32::try_from(days.max(1)).unwrap_or(u32::MAX)
}

/// Budget in percentage points of the weekly limit.
///
/// `w0` is the weekly utilisation at the start of the local day. A `w0`
/// above the current value means the window reset unnoticed and restarts
/// the baseline at 0.
pub fn weekly_budget(w_now: f64, w0: f64, days_left: u32, resets_at: EpochMs) -> DailyBudget {
    let w0 = if w0 > w_now { 0.0 } else { w0 };
    let allowance = ((100.0 - w0) / f64::from(days_left.max(1))).max(0.0);
    let used_today = w_now - w0;
    DailyBudget {
        basis: BudgetBasis::WeeklyLimit,
        allowance,
        used_today,
        used_percent: percent(used_today, allowance),
        days_left,
        resets_at: Some(resets_at),
        // Needs the token history; filled in by `weekly_from_db`.
        tokens_left_per_day: None,
    }
}

/// Headline tokens that may still be used per day until the weekly reset
/// (estimate: the weekly limit also counts claude.ai chats).
///
/// The ratio is tokens per weekly percent point: `window_tokens` (headline
/// tokens since the weekly window start) divided by `w_now`. `None` at 0 %
/// utilisation or without tokens in the window.
pub fn tokens_left_per_day(w_now: f64, days_left: u32, window_tokens: u64) -> Option<f64> {
    if w_now <= 0.0 || window_tokens == 0 {
        return None;
    }
    let per_point = window_tokens as f64 / w_now;
    let left = (100.0 - w_now) / f64::from(days_left.max(1)) * per_point;
    Some(left.max(0.0))
}

/// Budget in headline tokens from the average active day of the previous
/// seven local days. `None` without an active day.
pub fn history_budget(previous_days: &[u64], today_tokens: u64) -> Option<DailyBudget> {
    let active: Vec<u64> = previous_days.iter().copied().filter(|&t| t > 0).collect();
    if active.is_empty() {
        return None;
    }
    let allowance = active.iter().sum::<u64>() as f64 / active.len() as f64;
    let used_today = today_tokens as f64;
    Some(DailyBudget {
        basis: BudgetBasis::TokenHistory,
        allowance,
        used_today,
        used_percent: percent(used_today, allowance),
        days_left: 1,
        resets_at: None,
        tokens_left_per_day: None,
    })
}

/// Weekly-limit budget from the stored snapshots, if the weekly reading
/// carries a reset time that is still ahead.
pub fn weekly_from_db<Tz: TimeZone>(
    db: &Db,
    weekly: &crate::contracts::limits::LimitWindow,
    now: EpochMs,
    tz: &Tz,
) -> DbResult<Option<DailyBudget>> {
    let Some(resets_at) = weekly.resets_at.filter(|&r| r > now) else {
        return Ok(None);
    };
    let today = local_date(tz, now);
    let start = day_start(tz, today);
    let window_start = resets_at.saturating_sub(WEEK_MS);
    let w0 = if window_start > start {
        // The weekly window restarted during today.
        0.0
    } else {
        db.weekly_baseline(start, Some(window_start))?
            .unwrap_or(weekly.utilization)
    };
    let left = days_left(today, local_date(tz, resets_at));
    let mut budget = weekly_budget(weekly.utilization, w0, left, resets_at);
    let window_tokens = db
        .tokens_between(window_start, now.saturating_add(1))?
        .headline();
    budget.tokens_left_per_day = tokens_left_per_day(weekly.utilization, left, window_tokens);
    Ok(Some(budget))
}

/// Token-history budget from the database (previous days, today excluded).
pub fn history_from_db<Tz: TimeZone>(
    db: &Db,
    today_tokens: u64,
    now: EpochMs,
    tz: &Tz,
) -> DbResult<Option<DailyBudget>> {
    let today = local_date(tz, now);
    let from = day_start(tz, add_days(today, -HISTORY_DAYS));
    let to = day_start(tz, today);
    let days = db.tokens_by_local_day(tz, from, to)?;
    let tokens: Vec<u64> = days.iter().map(|d| d.totals.headline()).collect();
    Ok(history_budget(&tokens, today_tokens))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn days_left_counts_both_ends_and_floors_at_one() {
        assert_eq!(days_left(d(2026, 10, 28), d(2026, 10, 28)), 1);
        assert_eq!(days_left(d(2026, 10, 28), d(2026, 10, 31)), 4);
        assert_eq!(days_left(d(2026, 10, 28), d(2026, 10, 20)), 1);
    }

    #[test]
    fn weekly_budget_spreads_the_remaining_percent() {
        // 30 % at the start of the day, 4 days left: 17.5 points per day.
        let b = weekly_budget(37.0, 30.0, 4, 123);
        assert_eq!(b.basis, BudgetBasis::WeeklyLimit);
        assert!((b.allowance - 17.5).abs() < 1e-9);
        assert!((b.used_today - 7.0).abs() < 1e-9);
        assert!((b.used_percent - 40.0).abs() < 1e-9);
        assert_eq!((b.days_left, b.resets_at), (4, Some(123)));
        // Over budget exceeds 100.
        assert!(weekly_budget(60.0, 30.0, 4, 1).used_percent > 100.0);
    }

    #[test]
    fn weekly_budget_handles_reset_and_exhausted_week() {
        // Baseline above the current value: the window restarted.
        let b = weekly_budget(5.0, 90.0, 7, 1);
        assert!((b.allowance - 100.0 / 7.0).abs() < 1e-9);
        assert!((b.used_today - 5.0).abs() < 1e-9);
        // Nothing left: no division by zero.
        let b = weekly_budget(100.0, 100.0, 2, 1);
        assert_eq!(b.allowance, 0.0);
        assert_eq!(b.used_percent, 100.0);
        assert!(b.used_percent.is_finite());
    }

    #[test]
    fn tokens_left_per_day_scales_by_tokens_per_point() {
        // 40 % used with 4 000 tokens: 100 tokens per point; 60 points over
        // 3 days is 20 points per day, so 2 000 tokens per day.
        let v = tokens_left_per_day(40.0, 3, 4000).unwrap();
        assert!((v - 2000.0).abs() < 1e-9);
        // Over the limit never goes negative.
        assert_eq!(tokens_left_per_day(120.0, 2, 500), Some(0.0));
        // No utilisation or no tokens: unavailable.
        assert_eq!(tokens_left_per_day(0.0, 3, 4000), None);
        assert_eq!(tokens_left_per_day(40.0, 3, 0), None);
    }

    #[test]
    fn history_budget_has_no_tokens_left_per_day() {
        assert_eq!(
            history_budget(&[1000], 1).unwrap().tokens_left_per_day,
            None
        );
    }

    #[test]
    fn history_budget_averages_active_days_only() {
        assert_eq!(history_budget(&[], 10), None);
        assert_eq!(history_budget(&[0, 0], 10), None);
        let b = history_budget(&[1000, 0, 3000], 500).unwrap();
        assert_eq!(b.basis, BudgetBasis::TokenHistory);
        assert_eq!(b.allowance, 2000.0);
        assert_eq!(b.used_today, 500.0);
        assert_eq!(b.used_percent, 25.0);
        assert_eq!((b.days_left, b.resets_at), (1, None));
    }
}
