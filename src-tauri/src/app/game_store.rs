//! [`GameStore`] over the database helpers (`xp_events`, `achievements`,
//! `app_state.retro_done`).

use serde_json::json;

use crate::contracts::{AchievementId, EpochMs};
use crate::db::{AchievementRow, Db, DbError, XpEvent};
use crate::game::{AchievementRecord, GameData, GameStore, LedgerEntry};

const KEY_RETRO_DONE: &str = "retro_done";
const KEY_RULES_VERSION: &str = "game_rules_version";

/// Borrowed adapter; create one per evaluation.
pub struct DbGameStore<'a> {
    db: &'a Db,
}

impl<'a> DbGameStore<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }
}

impl DbGameStore<'_> {
    /// Runs the rules-version recount as one unit: a failure rolls every
    /// step back, so a crash cannot leave a half-recounted ledger. The
    /// inner writes use savepoints, which nest inside this one.
    pub fn recount_if_outdated(&mut self) -> Result<bool, DbError> {
        savepoint(self.db, |_| crate::game::recount_if_outdated(self))
    }
}

/// Runs `f` inside a SQLite savepoint (nestable, unlike `BEGIN`): released
/// on success, rolled back on error.
fn savepoint<T>(db: &Db, f: impl FnOnce(&Db) -> Result<T, DbError>) -> Result<T, DbError> {
    db.conn().execute_batch("SAVEPOINT tkm_game")?;
    match f(db) {
        Ok(v) => {
            db.conn().execute_batch("RELEASE tkm_game")?;
            Ok(v)
        }
        Err(e) => {
            let _ = db
                .conn()
                .execute_batch("ROLLBACK TO tkm_game; RELEASE tkm_game");
            Err(e)
        }
    }
}

fn achievement_id(id: &str) -> Option<AchievementId> {
    AchievementId::ALL.into_iter().find(|a| a.as_str() == id)
}

fn to_u32(v: i64) -> u32 {
    u32::try_from(v.max(0)).unwrap_or(u32::MAX)
}

impl GameStore for DbGameStore<'_> {
    type Error = DbError;

    fn load(&self) -> Result<GameData, DbError> {
        let ledger = self
            .db
            .xp_events()?
            .into_iter()
            .map(|e| LedgerEntry {
                earned_at: e.earned_at,
                mode: e.mode,
                kind: e.kind,
                reference: e.reference,
                amount: to_u32(e.amount),
                retroactive: e.retroactive,
            })
            .collect();
        // Rows with an id this build does not know are ignored.
        let achievements = self
            .db
            .achievements()?
            .into_iter()
            .filter_map(|r| {
                Some(AchievementRecord {
                    id: achievement_id(&r.id)?,
                    unlocked_at: r.unlocked_at,
                    retroactive: r.retroactive,
                    current: r.current.map(to_u32),
                    target: r.target.map(to_u32),
                    updated_at: r.updated_at,
                })
            })
            .collect();
        let retro_done = self
            .db
            .app_state(KEY_RETRO_DONE)?
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok(GameData {
            ledger,
            achievements,
            retro_done,
        })
    }

    fn append_ledger(&mut self, entries: &[LedgerEntry]) -> Result<(), DbError> {
        savepoint(self.db, |db| {
            for e in entries {
                db.insert_xp_event(&XpEvent {
                    earned_at: e.earned_at,
                    mode: e.mode,
                    kind: e.kind.clone(),
                    reference: e.reference.clone(),
                    amount: i64::from(e.amount),
                    retroactive: e.retroactive,
                })?;
            }
            Ok(())
        })
    }

    fn upsert_achievements(&mut self, records: &[AchievementRecord]) -> Result<(), DbError> {
        savepoint(self.db, |db| {
            for r in records {
                let at: Option<EpochMs> = r.unlocked_at;
                db.upsert_achievement(&AchievementRow {
                    id: r.id.as_str().to_owned(),
                    unlocked_at: at,
                    retroactive: r.retroactive,
                    current: r.current.map(i64::from),
                    target: r.target.map(i64::from),
                    updated_at: r.updated_at,
                })?;
            }
            Ok(())
        })
    }

    fn set_retro_done(&mut self, done: bool) -> Result<(), DbError> {
        self.db.set_app_state(KEY_RETRO_DONE, &json!(done))
    }

    fn rules_version(&self) -> Result<u32, DbError> {
        // Unset (fresh or pre-recount database) reports 0.
        Ok(self
            .db
            .app_state(KEY_RULES_VERSION)?
            .and_then(|v| v.as_u64())
            .map_or(0, |v| u32::try_from(v).unwrap_or(u32::MAX)))
    }

    fn set_rules_version(&mut self, version: u32) -> Result<(), DbError> {
        self.db.set_app_state(KEY_RULES_VERSION, &json!(version))
    }

    fn delete_ledger_kinds(&mut self, kinds: &[&str]) -> Result<(), DbError> {
        savepoint(self.db, |db| {
            for kind in kinds {
                db.conn()
                    .execute("DELETE FROM xp_events WHERE kind = ?1", [kind])?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Mode;
    use crate::game::{evaluate, DayFacts, Facts};
    use chrono::{FixedOffset, NaiveDate};

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    #[test]
    fn roundtrips_ledger_achievements_and_retro_flag() {
        let db = Db::open_in_memory().unwrap();
        let mut store = DbGameStore::new(&db);
        assert_eq!(store.load().unwrap(), GameData::default());

        let day = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        let ts = 1_793_232_000_000;
        let facts = Facts {
            days: vec![DayFacts::from_usage(day, [(ts, 250_000)], None, &tz())],
            ..Facts::default()
        };
        let now = ts + 3_600_000;
        let ev = evaluate(&GameData::default(), &facts, Mode::Max, now, &tz(), false);
        assert!(!ev.ledger.is_empty() && !ev.achievements.is_empty());
        store.append_ledger(&ev.ledger).unwrap();
        store.upsert_achievements(&ev.achievements).unwrap();
        store.set_retro_done(true).unwrap();

        let data = store.load().unwrap();
        assert_eq!(data.ledger, ev.ledger);
        assert_eq!(data.achievements, ev.achievements);
        assert!(data.retro_done);

        // Repeating the append changes nothing (unique kind/ref).
        store.append_ledger(&ev.ledger).unwrap();
        assert_eq!(store.load().unwrap().ledger.len(), ev.ledger.len());
    }

    #[test]
    fn unknown_achievement_ids_are_ignored() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_achievement(&AchievementRow {
            id: "fromTheFuture".into(),
            unlocked_at: Some(1),
            retroactive: false,
            current: None,
            target: None,
            updated_at: 1,
        })
        .unwrap();
        assert!(DbGameStore::new(&db)
            .load()
            .unwrap()
            .achievements
            .is_empty());
    }

    #[test]
    fn rules_version_and_kind_delete() {
        let db = Db::open_in_memory().unwrap();
        let mut store = DbGameStore::new(&db);
        assert_eq!(store.rules_version().unwrap(), 0);
        store.set_rules_version(2).unwrap();
        assert_eq!(store.rules_version().unwrap(), 2);

        let row = |kind: &str, amount| LedgerEntry {
            earned_at: 1,
            mode: Mode::Max,
            kind: kind.to_owned(),
            reference: "r".to_owned(),
            amount,
            retroactive: false,
        };
        store
            .append_ledger(&[row("tokens", 5), row("day", 7), row("daymode", 0)])
            .unwrap();
        store.delete_ledger_kinds(&["tokens", "day"]).unwrap();
        let left = store.load().unwrap().ledger;
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].kind, "daymode");
        assert_eq!(left[0].amount, 0);
    }

    #[test]
    fn recount_runs_once_and_rolls_back_on_failure() {
        let db = Db::open_in_memory().unwrap();
        let mut store = DbGameStore::new(&db);
        store
            .append_ledger(&[LedgerEntry {
                earned_at: 1,
                mode: Mode::Max,
                kind: "tokens".to_owned(),
                reference: "2026-10-01/a".to_owned(),
                amount: 5,
                retroactive: false,
            }])
            .unwrap();
        store.set_retro_done(true).unwrap();
        assert!(store.recount_if_outdated().unwrap());
        assert!(!store.recount_if_outdated().unwrap());
        let data = store.load().unwrap();
        assert!(!data.retro_done);
        assert_eq!(data.ledger.len(), 1);
        assert_eq!(data.ledger[0].kind, "daymode");

        let r: Result<(), DbError> = savepoint(&db, |d| {
            d.set_app_state("x", &json!(1))?;
            d.conn().execute_batch("SELECT * FROM missing_table")?;
            Ok(())
        });
        assert!(r.is_err());
        assert_eq!(db.app_state("x").unwrap(), None);
    }
}
