//! Storage seam of the game. The module never touches SQL; the app layer
//! implements [`GameStore`] on top of the database helpers (`xp_events`,
//! `achievements`, `app_state.retro_done`). [`MemoryStore`] backs the tests.

use std::collections::HashSet;
use std::convert::Infallible;

use chrono::TimeZone;

use super::eval::evaluate;
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
}
