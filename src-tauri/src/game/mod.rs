//! Gamification: XP ledger rules, level curve, titles, achievements and the
//! retroactive replay.
//!
//! - [`constants`]: every number and string of the rules.
//! - [`levels`]: level curve and titles.
//! - [`evaluate`]: the pure evaluation `(stored data, facts, mode, now, tz) ->
//!   (state, ledger rows, unlocks, level-up)`.
//! - [`GameStore`]: the storage seam; the app layer implements it on the
//!   database. [`run_evaluation`] ties load, evaluate and persist together.
//!
//! The first-start replay is the same [`evaluate`] over the whole history
//! with `retroactive = true`; it yields the same ledger and unlocks as
//! evaluating live after every change.

pub mod constants;
mod eval;
pub mod levels;
mod model;
mod store;

#[cfg(test)]
mod tests;

pub use eval::{day_start_ms, diy_day_xp, evaluate, local_date, max_block_xp};
pub use model::{
    in_night_window, AchievementRecord, DayFacts, Evaluation, Facts, GameData, LedgerEntry,
    LimitFact,
};
pub use store::{persist, recount_if_outdated, run_evaluation, GameStore, MemoryStore};
