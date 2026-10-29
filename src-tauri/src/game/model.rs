//! Data the game module reads and produces. Plain structs, no storage and no
//! time source: the caller supplies facts, the stored game data and "now".

use chrono::{NaiveDate, TimeZone, Timelike};

use super::constants::{NIGHT_END_HOUR, NIGHT_START_HOUR};
use crate::contracts::{AchievementId, AchievementUnlocked, EpochMs, GameState, LevelUp, Mode};

/// One row of the XP ledger (`xp_events`). `(kind, reference)` is unique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    pub earned_at: EpochMs,
    /// The mode the XP was earned in. Never `NoneYet`.
    pub mode: Mode,
    pub kind: String,
    /// The `ref` column.
    pub reference: String,
    pub amount: u32,
    pub retroactive: bool,
}

/// One row of the `achievements` table. Absent row = locked, no progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementRecord {
    pub id: AchievementId,
    pub unlocked_at: Option<EpochMs>,
    pub retroactive: bool,
    pub current: Option<u32>,
    pub target: Option<u32>,
    pub updated_at: EpochMs,
}

/// Everything stored about the game, as loaded by a [`super::GameStore`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameData {
    pub ledger: Vec<LedgerEntry>,
    pub achievements: Vec<AchievementRecord>,
    /// The first-start history replay has run (`app_state.retro_done`).
    pub retro_done: bool,
}

/// What happened on one local calendar day.
///
/// Build it with [`DayFacts::from_usage`]. `tokens` are headline tokens
/// (input + output + cache creation; cache reads are excluded as everywhere).
#[derive(Debug, Clone, PartialEq)]
pub struct DayFacts {
    pub day: NaiveDate,
    /// Headline tokens of the day (so far, for the current day).
    pub tokens: u64,
    /// Earliest usage record of the day.
    pub first_at: Option<EpochMs>,
    /// Earliest usage record inside the Night Owl window.
    pub night_at: Option<EpochMs>,
    /// Do it yourself: how much of the daily budget the day used, in percent
    /// (`DailyBudget::used_percent` of that day). `None` without a budget.
    pub budget_used_percent: Option<f64>,
}

impl DayFacts {
    /// Facts of `day` from its usage records `(timestamp, headline tokens)`.
    /// Every record of the day must be included; the records are not checked
    /// against `day`.
    pub fn from_usage<Tz: TimeZone>(
        day: NaiveDate,
        usage: impl IntoIterator<Item = (EpochMs, u64)>,
        budget_used_percent: Option<f64>,
        tz: &Tz,
    ) -> Self {
        let mut facts = DayFacts {
            day,
            tokens: 0,
            first_at: None,
            night_at: None,
            budget_used_percent,
        };
        for (ts, tokens) in usage {
            facts.tokens += tokens;
            facts.first_at = Some(facts.first_at.map_or(ts, |f| f.min(ts)));
            if in_night_window(ts, tz) {
                facts.night_at = Some(facts.night_at.map_or(ts, |f| f.min(ts)));
            }
        }
        facts
    }
}

/// `true` if the local time of `ts` lies in the Night Owl window.
pub fn in_night_window<Tz: TimeZone>(ts: EpochMs, tz: &Tz) -> bool {
    tz.timestamp_millis_opt(ts)
        .single()
        .is_some_and(|dt| (NIGHT_START_HOUR..NIGHT_END_HOUR).contains(&dt.hour()))
}

/// A stored limit reading (`limit_snapshots`), reduced to what the game uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimitFact {
    pub at: EpochMs,
    /// 5-hour utilisation in percent (may exceed 100).
    pub five_hour_pct: Option<f64>,
    pub five_hour_resets_at: Option<EpochMs>,
}

/// New facts for one evaluation. Facts are complete for the units they
/// touch rather than deltas, which keeps evaluation stateless:
///
/// - `days`: a contiguous range of local days ending with the current day
///   (the day of `now`), oldest first, idle days included (`tokens = 0`).
///   Live: at least [`super::constants::STREAK_LOOKBACK_DAYS`] days; replay:
///   from the first day with usage. Finished days award their XP once.
/// - `limits`: every reading of each 5-hour window the evaluation should
///   look at (live: all readings of the last 12 hours; replay: all). Extra
///   readings are harmless.
/// - `boo_at`: set by [`super::easter_egg`] only.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    pub days: Vec<DayFacts>,
    pub limits: Vec<LimitFact>,
    pub boo_at: Option<EpochMs>,
}

/// Result of one evaluation. The caller persists `ledger` and `achievements`
/// (see [`super::run_evaluation`]) and emits events for `unlocks` and
/// `level_up` unless the evaluation was retroactive.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// The state after the evaluation.
    pub state: GameState,
    /// New ledger rows (never contains a `(kind, ref)` already stored).
    pub ledger: Vec<LedgerEntry>,
    /// Achievement rows to upsert: new unlocks and changed progress.
    pub achievements: Vec<AchievementRecord>,
    /// New unlocks, oldest first.
    pub unlocks: Vec<AchievementUnlocked>,
    /// At most one: a single event covers several levels at once.
    pub level_up: Option<LevelUp>,
}
