//! Pure game evaluation: stored game data + new facts -> new state, ledger
//! rows, unlocks and level-up. No I/O, no clock, no time zone lookup: "now"
//! and the zone are parameters.
//!
//! Idempotency: every XP row has a unique `(kind, ref)` and the evaluation
//! never emits a key that is already stored; an achievement that is already
//! unlocked is never unlocked again. Evaluating the same facts twice, or the
//! full history at once versus step by step, ends in the same ledger.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{NaiveDate, TimeZone, Timelike};

use super::constants::{
    achievement_def, AchievementDef, ACHIEVEMENTS, FULL_RATE_BLOCKS_PER_DAY, KIND_ACHIEVEMENT,
    KIND_DAY, KIND_DAY_LEGACY, KIND_DAY_MODE, KIND_TOKENS, MAX_ACTIVE_DAY_XP, RAMBO_PERCENT,
    RECENT_UNLOCKS, SOFT_BAND_END_BLOCK, SO_CLOSE_PERCENT, TOKENS_PER_BLOCK, WINDOW_BUCKET_MS,
    XP_PER_BLOCK, XP_PER_SOFT_BLOCK,
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
/// - `mode`: `NoneYet` (before onboarding) awards nothing; any other value
///   earns XP the one way there is.
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
    let earning = mode != Mode::NoneYet;
    let mode = if earning { Mode::Max } else { Mode::NoneYet };

    let xp_before: u64 = prev.ledger.iter().map(|e| u64::from(e.amount)).sum();
    let mut awards = Awards::new(prev, retroactive);

    // XP for burning.
    if earning {
        let day_modes = day_modes(prev);
        let marker_only = marker_only_days(prev);
        let mut days: Vec<&DayFacts> = facts.days.iter().filter(|d| d.tokens > 0).collect();
        days.sort_by_key(|d| d.day);
        for d in days {
            let key = d.day.to_string();
            // A day already awarded under the retired Do it yourself mode
            // stays as it was paid; a recounted marker-only day is skipped
            // the same way.
            let legacy = match marker_only.get(&key) {
                Some(marked) => *marked != Mode::Max,
                None => day_modes.get(&key).is_some_and(|dm| *dm != Mode::Max),
            };
            if legacy {
                continue;
            }
            let at = day_start_ms(d.day, tz);
            let blocks = (d.tokens / TOKENS_PER_BLOCK).min(SOFT_BAND_END_BLOCK);
            for k in 1..=blocks {
                awards.add(mode, at, KIND_TOKENS, format!("{key}/{k}"), max_block_xp(k));
            }
            awards.add(mode, at, KIND_DAY, key, MAX_ACTIVE_DAY_XP);
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
    if earning {
        for (idx, def) in ACHIEVEMENTS.iter().enumerate() {
            if def.legacy || is_unlocked(&records, def.id) {
                continue;
            }
            let at = match def.id {
                AchievementId::FirstBlood => facts.days.iter().filter_map(|d| d.first_at).min(),
                AchievementId::NightOwl => facts.days.iter().filter_map(|d| d.night_at).min(),
                AchievementId::RateLimitRambo => rambo_at(&facts.limits),
                AchievementId::SoClose => so_close_at(&facts.limits, now),
                // Legacy badges: kept for users who unlocked them, never earned again.
                AchievementId::TokenMonk | AchievementId::Boo => None,
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
        awards.add(
            mode,
            at,
            KIND_ACHIEVEMENT,
            def.id.as_str().to_string(),
            def.xp,
        );
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
        title: title_for(after.level).to_string(),
        title_changed: title_for(after.level) != title_for(before.level),
        retroactive,
    });

    let state = build_state(mode, &records, xp_after, prev.retro_done || retroactive, tz);

    Evaluation {
        state,
        ledger: new_ledger,
        achievements: changed,
        unlocks,
        level_up,
    }
}

/// XP of the `block`-th (1-based) 100k-token block of a day: full
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
    retroactive: bool,
}

impl Awards {
    fn new(prev: &GameData, retroactive: bool) -> Self {
        Self {
            seen: prev
                .ledger
                .iter()
                .map(|e| (e.kind.clone(), e.reference.clone()))
                .collect(),
            new: Vec::new(),
            retroactive,
        }
    }

    fn add(&mut self, mode: Mode, earned_at: EpochMs, kind: &str, reference: String, amount: u32) {
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
/// rows). Only the retired Do it yourself mode differs from Max.
pub(super) fn day_modes(prev: &GameData) -> HashMap<String, Mode> {
    prev.ledger
        .iter()
        .filter(|e| {
            [KIND_TOKENS, KIND_DAY, KIND_DAY_LEGACY, KIND_DAY_MODE].contains(&e.kind.as_str())
        })
        .filter_map(|e| {
            let day = e.reference.split('/').next()?;
            Some((day.to_string(), e.mode))
        })
        .collect()
}

/// Days that only carry a recount `daymode` marker (no XP rows yet), with the
/// mode they were earned in.
fn marker_only_days(prev: &GameData) -> HashMap<String, Mode> {
    let paid: HashSet<&str> = prev
        .ledger
        .iter()
        .filter(|e| [KIND_TOKENS, KIND_DAY, KIND_DAY_LEGACY].contains(&e.kind.as_str()))
        .filter_map(|e| e.reference.split('/').next())
        .collect();
    prev.ledger
        .iter()
        .filter(|e| e.kind == KIND_DAY_MODE && !paid.contains(e.reference.as_str()))
        .map(|e| (e.reference.clone(), e.mode))
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
    retro_done: bool,
    tz: &Tz,
) -> GameState {
    let info = level_info(xp_total);

    let achievements: Vec<AchievementProgress> = ACHIEVEMENTS
        .iter()
        .filter(|def| {
            !def.legacy
                || records
                    .get(&def.id)
                    .is_some_and(|r| r.unlocked_at.is_some())
        })
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
        title: title_for(info.level).to_string(),
        xp_total,
        xp_into_level: info.into_level,
        xp_for_level: info.for_level,
        progress: info.progress(),
        recent_unlocks,
        achievements,
        retro_done,
    }
}
