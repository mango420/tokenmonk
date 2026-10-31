//! [`GameStore`] over the database helpers (`xp_events`, `achievements`,
//! `app_state.retro_done`).

use serde_json::json;

use crate::contracts::{AchievementId, EpochMs};
use crate::db::{AchievementRow, Db, DbError, XpEvent};
use crate::game::{AchievementRecord, GameData, GameStore, LedgerEntry};

const KEY_RETRO_DONE: &str = "retro_done";

/// Borrowed adapter; create one per evaluation.
pub struct DbGameStore<'a> {
    db: &'a Db,
}

impl<'a> DbGameStore<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
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
        let tx = self.db.conn().unchecked_transaction()?;
        for e in entries {
            self.db.insert_xp_event(&XpEvent {
                earned_at: e.earned_at,
                mode: e.mode,
                kind: e.kind.clone(),
                reference: e.reference.clone(),
                amount: i64::from(e.amount),
                retroactive: e.retroactive,
            })?;
        }
        tx.commit()?;
        Ok(())
    }

    fn upsert_achievements(&mut self, records: &[AchievementRecord]) -> Result<(), DbError> {
        let tx = self.db.conn().unchecked_transaction()?;
        for r in records {
            let at: Option<EpochMs> = r.unlocked_at;
            self.db.upsert_achievement(&AchievementRow {
                id: r.id.as_str().to_owned(),
                unlocked_at: at,
                retroactive: r.retroactive,
                current: r.current.map(i64::from),
                target: r.target.map(i64::from),
                updated_at: r.updated_at,
            })?;
        }
        tx.commit()?;
        Ok(())
    }

    fn set_retro_done(&mut self, done: bool) -> Result<(), DbError> {
        self.db.set_app_state(KEY_RETRO_DONE, &json!(done))
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
}
