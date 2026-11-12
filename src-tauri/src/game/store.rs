//! Storage seam of the game. The module never touches SQL; the app layer
//! implements [`GameStore`] on top of the database helpers (`xp_events`,
//! `achievements`, `app_state.retro_done`). [`MemoryStore`] backs the tests.

use std::collections::{BTreeMap, HashSet};
use std::convert::Infallible;

use chrono::TimeZone;

use super::constants::{KIND_DAY_MODE, RECOUNT_DROP_KINDS, RULES_VERSION};
use super::eval::{day_modes, evaluate};
use super::model::{AchievementRecord, Evaluation, Facts, GameData, LedgerEntry};
use crate::contracts::{EpochMs, Mode};

pub trait GameStore {
    type Error: std::error::Error;

    /// The full ledger, all achievement rows and the retro flag.
    fn load(&self) -> Result<GameData, Self::Error>;

    /// Append rows. Rows whose `(kind, ref)` exists are skipped silently
    /// (`INSERT OR IGNORE`), which makes a repeated call harmless.
    fn append_ledger(&mut self, entries: &[LedgerEntry]) -> Result<(), Self::Error>;

    /// Insert or replace achievement rows by id.
    fn upsert_achievements(&mut self, records: &[AchievementRecord]) -> Result<(), Self::Error>;

    /// Persist `app_state.retro_done`.
    fn set_retro_done(&mut self, done: bool) -> Result<(), Self::Error>;

    /// The rules version the ledger was built with. The default reports the
    /// current version, which keeps the recount inert for stores that do not
    /// persist it. A fresh database reports 0 (or any older version).
    fn rules_version(&self) -> Result<u32, Self::Error> {
        Ok(RULES_VERSION)
    }

    /// Persist the rules version the ledger now follows. Default: no-op.
    fn set_rules_version(&mut self, _version: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Delete every ledger row whose `kind` is in `kinds`. Default: no-op.
    fn delete_ledger_kinds(&mut self, _kinds: &[&str]) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Recount the ledger once per rules version. Call it before reading
/// `retro_done`. When the stored version is older than `RULES_VERSION`:
/// every day keeps the mode it was earned in (zero-XP `daymode` rows), the
/// `tokens` and `day` rows are dropped, achievements and their XP stay,
/// `retro_done` is cleared so the next evaluation is a retroactive replay
/// (no toasts, one summary), and the version is stored. Returns whether a
/// recount ran. Idempotent.
pub fn recount_if_outdated<S: GameStore>(store: &mut S) -> Result<bool, S::Error> {
    if store.rules_version()? >= RULES_VERSION {
        return Ok(false);
    }
    let data = store.load()?;
    let existing = day_modes(&GameData {
        ledger: data
            .ledger
            .iter()
            .filter(|e| e.kind == KIND_DAY_MODE)
            .cloned()
            .collect(),
        ..GameData::default()
    });
    let mut markers: BTreeMap<String, LedgerEntry> = BTreeMap::new();
    for e in data
        .ledger
        .iter()
        .filter(|e| RECOUNT_DROP_KINDS.contains(&e.kind.as_str()))
    {
        let Some(day) = e.reference.split('/').next() else {
            continue;
        };
        if existing.contains_key(day) {
            continue;
        }
        markers
            .entry(day.to_string())
            .and_modify(|m| m.earned_at = m.earned_at.min(e.earned_at))
            .or_insert_with(|| LedgerEntry {
                earned_at: e.earned_at,
                mode: e.mode,
                kind: KIND_DAY_MODE.to_string(),
                reference: day.to_string(),
                amount: 0,
                retroactive: true,
            });
    }
    let markers: Vec<LedgerEntry> = markers.into_values().collect();
    store.append_ledger(&markers)?;
    store.delete_ledger_kinds(&RECOUNT_DROP_KINDS)?;
    store.set_retro_done(false)?;
    store.set_rules_version(RULES_VERSION)?;
    Ok(true)
}

/// Load, evaluate, persist. The caller emits events for the returned unlocks
/// and level-up unless `retroactive`, then `game-updated` with the state.
/// When `retroactive` is set the retro flag is stored, so the replay runs once.
pub fn run_evaluation<S: GameStore, Tz: TimeZone>(
    store: &mut S,
    facts: &Facts,
    mode: Mode,
    now: EpochMs,
    tz: &Tz,
    retroactive: bool,
) -> Result<Evaluation, S::Error> {
    let prev = store.load()?;
    let evaluation = evaluate(&prev, facts, mode, now, tz, retroactive);
    persist(store, &prev, &evaluation, retroactive)?;
    Ok(evaluation)
}

/// Write an evaluation to the store.
pub fn persist<S: GameStore>(
    store: &mut S,
    prev: &GameData,
    evaluation: &Evaluation,
    retroactive: bool,
) -> Result<(), S::Error> {
    store.append_ledger(&evaluation.ledger)?;
    store.upsert_achievements(&evaluation.achievements)?;
    if retroactive && !prev.retro_done {
        store.set_retro_done(true)?;
    }
    Ok(())
}

/// In-memory [`GameStore`] with the uniqueness of the real table.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    pub data: GameData,
    /// Stored rules version; `None` behaves like the current one.
    pub rules_version: Option<u32>,
}

impl GameStore for MemoryStore {
    type Error = Infallible;

    fn load(&self) -> Result<GameData, Self::Error> {
        Ok(self.data.clone())
    }

    fn append_ledger(&mut self, entries: &[LedgerEntry]) -> Result<(), Self::Error> {
        let mut seen: HashSet<(String, String)> = self
            .data
            .ledger
            .iter()
            .map(|e| (e.kind.clone(), e.reference.clone()))
            .collect();
        for e in entries {
            if seen.insert((e.kind.clone(), e.reference.clone())) {
                self.data.ledger.push(e.clone());
            }
        }
        Ok(())
    }

    fn upsert_achievements(&mut self, records: &[AchievementRecord]) -> Result<(), Self::Error> {
        for r in records {
            match self.data.achievements.iter_mut().find(|a| a.id == r.id) {
                Some(slot) => *slot = r.clone(),
                None => self.data.achievements.push(r.clone()),
            }
        }
        Ok(())
    }

    fn set_retro_done(&mut self, done: bool) -> Result<(), Self::Error> {
        self.data.retro_done = done;
        Ok(())
    }

    fn rules_version(&self) -> Result<u32, Self::Error> {
        Ok(self.rules_version.unwrap_or(RULES_VERSION))
    }

    fn set_rules_version(&mut self, version: u32) -> Result<(), Self::Error> {
        self.rules_version = Some(version);
        Ok(())
    }

    fn delete_ledger_kinds(&mut self, kinds: &[&str]) -> Result<(), Self::Error> {
        self.data
            .ledger
            .retain(|e| !kinds.contains(&e.kind.as_str()));
        Ok(())
    }
}
