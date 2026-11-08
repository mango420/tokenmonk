//! Pure game evaluation: stored game data + new facts -> new state, ledger
//! rows, unlocks and level-up. No I/O, no clock, no time zone lookup: "now"
//! and the zone are parameters.
//!
//! Idempotency: every XP row has a unique `(kind, ref)` and the evaluation
//! never emits a key that is already stored; an achievement that is already
//! unlocked is never unlocked again. Evaluating the same facts twice, or the
//! full history at once versus step by step, ends in the same ledger.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Days, NaiveDate, TimeZone, Timelike};

use super::constants::{
    achievement_def, AchievementDef, ACHIEVEMENTS, DIY_DAILY_MAX_XP, DIY_DAILY_MIN_XP,
    DIY_FULL_XP_UP_TO_PERCENT, DIY_UNMEASURED_DAY_XP, FULL_RATE_BLOCKS_PER_DAY, KIND_ACHIEVEMENT,
    KIND_DAY, KIND_DAY_MODE, KIND_TOKENS, MAX_ACTIVE_DAY_XP, RAMBO_PERCENT, RECENT_UNLOCKS,
    SOFT_BAND_END_BLOCK, SO_CLOSE_PERCENT, TOKENS_PER_BLOCK, TOKEN_MONK_BELOW_PERCENT,
    WINDOW_BUCKET_MS, XP_PER_BLOCK, XP_PER_SOFT_BLOCK,
};
use super::levels::{level_info, title_for};
use super::model::{
    AchievementRecord, DayFacts, Evaluation, Facts, GameData, LedgerEntry, LimitFact,
};
use crate::contracts::{
    AchievementId, AchievementProgress, AchievementUnlocked, EpochMs, GameState, LevelUp, Mode,
};

/// Evaluate new facts against the stored game data.
///
/// - `prev`: ledger, achievement rows and the retro flag as stored.
/// - `facts`: see [`Facts`].
/// - `mode`: the current mode. With `NoneYet` nothing is awarded.
/// - `now`: epoch ms; days before the local day of `now` are finished.
/// - `tz`: the local time zone, for calendar days and the Night Owl window.
/// - `retroactive`: the first-start replay. Marks every produced row and
///   unlock retroactive and sets `GameState::retro_done`.
pub fn evaluate<Tz: TimeZone>(
    prev: &GameData,
    facts: &Facts,
    mode: Mode,
    now: EpochMs,
    tz: &Tz,
    retroactive: bool,
) -> Evaluation {
    let today = local_date(now, tz);
    let earn_mode = (mode != Mode::NoneYet).then_some(mode);

    let xp_before: u64 = prev.ledger.iter().map(|e| u64::from(e.amount)).sum();
    let mut awards = Awards::new(prev, earn_mode, retroactive);

    // XP for burning (Max it out) or holding back (Do it yourself).
    if let Some(m) = earn_mode {
        let day_modes = day_modes(prev);
        let mut days: Vec<&DayFacts> = facts.days.iter().filter(|d| d.tokens > 0).collect();
        days.sort_by_key(|d| d.day);
        for d in days {
            let key = d.day.to_string();
            // A day already awarded under the other mode stays with it.
            if day_modes.get(&key).is_some_and(|dm| *dm != m) {
                continue;
            }
            let at = day_start_ms(d.day, tz);
            match m {
                Mode::Max => {
                    let blocks = (d.tokens / TOKENS_PER_BLOCK).min(SOFT_BAND_END_BLOCK);
                    for k in 1..=blocks {
                        awards.add(at, KIND_TOKENS, format!("{key}/{k}"), max_block_xp(k));
                    }
                    awards.add(at, KIND_DAY, key, MAX_ACTIVE_DAY_XP);
                }
                Mode::Diy if d.day < today => {
                    let xp = d
                        .budget_used_percent
                        .map_or(DIY_UNMEASURED_DAY_XP, diy_day_xp);
                    if xp > 0 {
                        awards.add(at, KIND_DAY, key, xp);
                    }
                }
                _ => {}
            }
        }
    }

    // Achievements.
    let mut records: HashMap<AchievementId, AchievementRecord> = prev
        .achievements
        .iter()
        .map(|r| (r.id, r.clone()))
        .collect();
    let is_unlocked = |records: &HashMap<AchievementId, AchievementRecord>, id| {
        records.get(&id).is_some_and(|r| r.unlocked_at.is_some())
    };

    let mut candidates: Vec<(EpochMs, usize, &'static AchievementDef)> = Vec::new();
    if let Some(m) = earn_mode {
        for (idx, def) in ACHIEVEMENTS.iter().enumerate() {
            if def.mode.is_some_and(|dm| dm != m) || is_unlocked(&records, def.id) {
                continue;
            }
            let at = match def.id {
                AchievementId::FirstBlood => facts.days.iter().filter_map(|d| d.first_at).min(),
                AchievementId::NightOwl => facts.days.iter().filter_map(|d| d.night_at).min(),
                AchievementId::RateLimitRambo => rambo_at(&facts.limits),
                AchievementId::SoClose => so_close_at(&facts.limits, now),
                AchievementId::TokenMonk => token_monk_at(&facts.days, today, tz),
                // Legacy badge: kept for users who unlocked it, never earned again.
                AchievementId::Boo => None,
            };
            if let Some(at) = at {
                candidates.push((at, idx, def));
            }
        }
    }
    candidates.sort_by_key(|(at, idx, _)| (*at, *idx));

    let mut changed: Vec<AchievementRecord> = Vec::new();
    let mut unlocks: Vec<AchievementUnlocked> = Vec::new();
    for (at, _, def) in candidates {
        awards.add(at, KIND_ACHIEVEMENT, def.id.as_str().to_string(), def.xp);
        let record = AchievementRecord {
            id: def.id,
            unlocked_at: Some(at),
            retroactive,
            current: def.target,
            target: def.target,
            updated_at: now,
        };
        records.insert(def.id, record.clone());
        changed.push(record);
        unlocks.push(unlocked_from(def, at, retroactive, tz));
    }

    // Level and state.
    let new_ledger = awards.new;
    let xp_after = xp_before + new_ledger.iter().map(|e| u64::from(e.amount)).sum::<u64>();
    let before = level_info(xp_before);
    let after = level_info(xp_after);
    let level_up = (after.level > before.level).then(|| LevelUp {
        level: after.level,
        previous_level: before.level,
        title: title_for(mode, after.level).to_string(),
        title_changed: title_for(mode, after.level) != title_for(mode, before.level),
        retroactive,
    });

    let preview = (mode == Mode::Diy)
        .then(|| {
            facts
                .days
                .iter()
                .find(|d| d.day == today && d.tokens > 0)
                .and_then(|d| d.budget_used_percent)
                .map(diy_day_xp)
        })
        .flatten();

    let state = build_state(
        mode,
        &records,
        xp_after,
        preview,
        prev.retro_done || retroactive,
        tz,
    );

    Evaluation {
        state,
        ledger: new_ledger,
        achievements: changed,
        unlocks,
        level_up,
    }
}

/// XP of the `block`-th (1-based) 100k-token block of a Max it out day: full
/// rate up to the full-rate band, the soft rate up to its end, 0 above.
pub fn max_block_xp(block: u64) -> u32 {
    if block <= FULL_RATE_BLOCKS_PER_DAY {
        XP_PER_BLOCK
    } else if block <= SOFT_BAND_END_BLOCK {
        XP_PER_SOFT_BLOCK
    } else {
        0
    }
}

/// XP a day at `used_percent` of the daily budget earns in Do it yourself:
/// the maximum up to the full-XP edge, linear down to the minimum at 100 %,
/// 0 above 100 %.
pub fn diy_day_xp(used_percent: f64) -> u32 {
    if used_percent > 100.0 {
        return 0;
    }
    if used_percent <= DIY_FULL_XP_UP_TO_PERCENT {
        return DIY_DAILY_MAX_XP;
    }
    let t = (used_percent - DIY_FULL_XP_UP_TO_PERCENT) / (100.0 - DIY_FULL_XP_UP_TO_PERCENT);
    let max = f64::from(DIY_DAILY_MAX_XP);
    (max - (max - f64::from(DIY_DAILY_MIN_XP)) * t).round() as u32
}

/// Local calendar day of an epoch timestamp.
pub fn local_date<Tz: TimeZone>(ts: EpochMs, tz: &Tz) -> NaiveDate {
    tz.timestamp_millis_opt(ts)
        .single()
        .map(|dt| dt.date_naive())
        .unwrap_or_default()
}

/// Epoch ms of the first instant of a local day (handles days without 00:00).
pub fn day_start_ms<Tz: TimeZone>(day: NaiveDate, tz: &Tz) -> EpochMs {
    for hour in 0..24 {
        let Some(naive) = day.and_hms_opt(hour, 0, 0) else {
            continue;
        };
        if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
            return dt.timestamp_millis();
        }
    }
    day.and_hms_opt(0, 0, 0)
        .map(|n| n.and_utc().timestamp_millis())
        .unwrap_or(0)
}

// --- internals -----------------------------------------------------------

/// Ledger rows to add, with the keys already taken.
struct Awards {
    seen: HashSet<(String, String)>,
    new: Vec<LedgerEntry>,
    mode: Option<Mode>,
    retroactive: bool,
}

impl Awards {
    fn new(prev: &GameData, mode: Option<Mode>, retroactive: bool) -> Self {
        Self {
            seen: prev
                .ledger
                .iter()
                .map(|e| (e.kind.clone(), e.reference.clone()))
                .collect(),
            new: Vec::new(),
            mode,
            retroactive,
        }
    }

    fn add(&mut self, earned_at: EpochMs, kind: &str, reference: String, amount: u32) {
        let Some(mode) = self.mode else { return };
        if amount == 0 || !self.seen.insert((kind.to_string(), reference.clone())) {
            return;
        }
        self.new.push(LedgerEntry {
            earned_at,
            mode,
            kind: kind.to_string(),
            reference,
            amount,
            retroactive: self.retroactive,
        });
    }
}

/// The mode each local day was awarded under (`tokens`, `day` and `daymode`
/// rows).
pub(super) fn day_modes(prev: &GameData) -> HashMap<String, Mode> {
    prev.ledger
        .iter()
        .filter(|e| e.kind == KIND_TOKENS || e.kind == KIND_DAY || e.kind == KIND_DAY_MODE)
        .filter_map(|e| {
            let day = e.reference.split('/').next()?;
            Some((day.to_string(), e.mode))
        })
        .collect()
}

fn rambo_at(limits: &[LimitFact]) -> Option<EpochMs> {
    limits
        .iter()
        .filter(|l| l.five_hour_pct.is_some_and(|p| p >= RAMBO_PERCENT))
        .map(|l| l.at)
        .min()
}

/// The end of the earliest finished 5-hour window that peaked in
/// `SO_CLOSE_PERCENT..RAMBO_PERCENT`.
fn so_close_at(limits: &[LimitFact], now: EpochMs) -> Option<EpochMs> {
    // window bucket -> (latest reset time, peak percent)
    let mut windows: BTreeMap<i64, (EpochMs, f64)> = BTreeMap::new();
    for l in limits {
        let (Some(pct), Some(resets)) = (l.five_hour_pct, l.five_hour_resets_at) else {
            continue;
        };
        let bucket = (resets + WINDOW_BUCKET_MS / 2).div_euclid(WINDOW_BUCKET_MS);
        let entry = windows.entry(bucket).or_insert((resets, pct));
        entry.0 = entry.0.max(resets);
        entry.1 = entry.1.max(pct);
    }
    windows
        .values()
        .filter(|(resets, peak)| {
            *resets <= now && *peak >= SO_CLOSE_PERCENT && *peak < RAMBO_PERCENT
        })
        .map(|(resets, _)| *resets)
        .min()
}

/// Token Monk: the end of the earliest finished active day that had a daily
/// budget and stayed below `TOKEN_MONK_BELOW_PERCENT` of it.
fn token_monk_at<Tz: TimeZone>(days: &[DayFacts], today: NaiveDate, tz: &Tz) -> Option<EpochMs> {
    days.iter()
        .filter(|d| d.day < today && d.tokens > 0)
        .filter(|d| {
            d.budget_used_percent
                .is_some_and(|p| p < TOKEN_MONK_BELOW_PERCENT)
        })
        .map(|d| d.day)
        .min()
        .and_then(|day| day.checked_add_days(Days::new(1)))
        .map(|next| day_start_ms(next, tz))
}

/// Context line of an unlock, derived from the id and the unlock time so it
/// needs no storage.
fn unlock_detail<Tz: TimeZone>(id: AchievementId, at: EpochMs, tz: &Tz) -> Option<String> {
    match id {
        AchievementId::FirstBlood | AchievementId::TokenMonk | AchievementId::Boo => None,
        AchievementId::NightOwl => {
            let dt = tz.timestamp_millis_opt(at).single()?;
            Some(format!("Coding at {:02}:{:02}", dt.hour(), dt.minute()))
        }
        AchievementId::RateLimitRambo => Some("5-hour limit hit".to_string()),
        AchievementId::SoClose => Some("Stopped just short of the limit".to_string()),
    }
}

fn unlocked_from<Tz: TimeZone>(
    def: &AchievementDef,
    at: EpochMs,
    retroactive: bool,
    tz: &Tz,
) -> AchievementUnlocked {
    AchievementUnlocked {
        id: def.id,
        name: def.name.to_string(),
        detail: unlock_detail(def.id, at, tz),
        xp: def.xp,
        unlocked_at: at,
        retroactive,
    }
}

fn build_state<Tz: TimeZone>(
    mode: Mode,
    records: &HashMap<AchievementId, AchievementRecord>,
    xp_total: u64,
    daily_xp_preview: Option<u32>,
    retro_done: bool,
    tz: &Tz,
) -> GameState {
    let info = level_info(xp_total);

    let achievements: Vec<AchievementProgress> = ACHIEVEMENTS
        .iter()
        .map(|def| {
            let rec = records.get(&def.id);
            let (current, target) = match def.target {
                Some(t) => (
                    Some(rec.and_then(|r| r.current).unwrap_or(0).min(t)),
                    Some(t),
                ),
                None => (None, None),
            };
            AchievementProgress {
                id: def.id,
                name: def.name.to_string(),
                description: def.description.to_string(),
                secret: def.secret,
                mode: def.mode,
                xp: def.xp,
                unlocked_at: rec.and_then(|r| r.unlocked_at),
                current,
                target,
                progress_label: None,
            }
        })
        .collect();

    let mut unlocked: Vec<&AchievementRecord> = records
        .values()
        .filter(|r| r.unlocked_at.is_some())
        .collect();
    unlocked.sort_by_key(|r| std::cmp::Reverse((r.unlocked_at, r.id.as_str())));
    let recent_unlocks = unlocked
        .into_iter()
        .take(RECENT_UNLOCKS)
        .filter_map(|r| {
            Some(unlocked_from(
                achievement_def(r.id),
                r.unlocked_at?,
                r.retroactive,
                tz,
            ))
        })
        .collect();

    GameState {
        mode,
        level: info.level,
        title: title_for(mode, info.level).to_string(),
        xp_total,
        xp_into_level: info.into_level,
        xp_for_level: info.for_level,
        progress: info.progress(),
        daily_xp_preview,
        recent_unlocks,
        achievements,
        retro_done,
    }
}
