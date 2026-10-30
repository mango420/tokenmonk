//! XP ledger and achievement state (tables `xp_events`, `achievements`).
//!
//! The game module decides what is awarded; this file only stores it.

use rusqlite::{params, OptionalExtension};

use super::{Db, DbError, DbResult};
use crate::contracts::settings::Mode;
use crate::contracts::EpochMs;

/// One row of the XP ledger. `(kind, reference)` is unique, which makes
/// awards idempotent (for example `("day", "2026-10-28")`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XpEvent {
    pub earned_at: EpochMs,
    /// `Max` or `Diy`; `NoneYet` is rejected on insert.
    pub mode: Mode,
    pub kind: String,
    /// Stored in column `ref`.
    pub reference: String,
    pub amount: i64,
    pub retroactive: bool,
}

/// Stored state of one achievement (`id` is an `AchievementId` string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementRow {
    pub id: String,
    pub unlocked_at: Option<EpochMs>,
    pub retroactive: bool,
    pub current: Option<i64>,
    pub target: Option<i64>,
    pub updated_at: EpochMs,
}

fn mode_str(mode: Mode) -> DbResult<&'static str> {
    match mode {
        Mode::Max => Ok("max"),
        Mode::Diy => Ok("diy"),
        Mode::NoneYet => Err(DbError::Invalid("xp event needs mode max or diy".into())),
    }
}

fn mode_from_str(s: &str) -> DbResult<Mode> {
    match s {
        "max" => Ok(Mode::Max),
        "diy" => Ok(Mode::Diy),
        other => Err(DbError::Invalid(format!("unknown xp mode {other:?}"))),
    }
}

impl Db {
    /// Inserts an XP event unless `(kind, ref)` exists already.
    /// Returns `true` if a row was added.
    pub fn insert_xp_event(&self, e: &XpEvent) -> DbResult<bool> {
        let n = self.conn().execute(
            "INSERT OR IGNORE INTO xp_events (earned_at, mode, kind, ref, amount, retroactive) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                e.earned_at,
                mode_str(e.mode)?,
                e.kind,
                e.reference,
                e.amount,
                e.retroactive
            ],
        )?;
        Ok(n > 0)
    }

    /// Sum of all XP amounts (level and progress derive from it).
    pub fn xp_total(&self) -> DbResult<i64> {
        Ok(self
            .conn()
            .query_row("SELECT COALESCE(SUM(amount), 0) FROM xp_events", [], |r| {
                r.get(0)
            })?)
    }

    /// The whole ledger in award order.
    pub fn xp_events(&self) -> DbResult<Vec<XpEvent>> {
        let mut stmt = self.conn().prepare(
            "SELECT earned_at, mode, kind, ref, amount, retroactive FROM xp_events \
             ORDER BY earned_at, id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, bool>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (earned_at, mode, kind, reference, amount, retroactive) = row?;
            out.push(XpEvent {
                earned_at,
                mode: mode_from_str(&mode)?,
                kind,
                reference,
                amount,
                retroactive,
            });
        }
        Ok(out)
    }

    /// Inserts or updates an achievement. An unlock is sticky: once
    /// `unlocked_at` is set, neither it nor `retroactive` change; progress
    /// (`current`, `target`) and `updated_at` always follow the new row.
    pub fn upsert_achievement(&self, a: &AchievementRow) -> DbResult<()> {
        self.conn().execute(
            "INSERT INTO achievements (id, unlocked_at, retroactive, current, target, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(id) DO UPDATE SET \
               retroactive = CASE WHEN achievements.unlocked_at IS NOT NULL \
                                  THEN achievements.retroactive ELSE excluded.retroactive END, \
               unlocked_at = COALESCE(achievements.unlocked_at, excluded.unlocked_at), \
               current = excluded.current, target = excluded.target, \
               updated_at = excluded.updated_at",
            params![
                a.id,
                a.unlocked_at,
                a.retroactive,
                a.current,
                a.target,
                a.updated_at
            ],
        )?;
        Ok(())
    }

    pub fn achievement(&self, id: &str) -> DbResult<Option<AchievementRow>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT id, unlocked_at, retroactive, current, target, updated_at \
                 FROM achievements WHERE id = ?1",
                [id],
                row_to_achievement,
            )
            .optional()?)
    }

    pub fn achievements(&self) -> DbResult<Vec<AchievementRow>> {
        let mut stmt = self.conn().prepare(
            "SELECT id, unlocked_at, retroactive, current, target, updated_at \
             FROM achievements ORDER BY id",
        )?;
        let rows = stmt.query_map([], row_to_achievement)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn row_to_achievement(r: &rusqlite::Row<'_>) -> rusqlite::Result<AchievementRow> {
    Ok(AchievementRow {
        id: r.get(0)?,
        unlocked_at: r.get(1)?,
        retroactive: r.get(2)?,
        current: r.get(3)?,
        target: r.get(4)?,
        updated_at: r.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xp(kind: &str, reference: &str, amount: i64) -> XpEvent {
        XpEvent {
            earned_at: 10,
            mode: Mode::Diy,
            kind: kind.into(),
            reference: reference.into(),
            amount,
            retroactive: false,
        }
    }

    #[test]
    fn xp_insert_is_idempotent_per_kind_and_ref() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.insert_xp_event(&xp("day", "2026-10-28", 40)).unwrap());
        assert!(!db.insert_xp_event(&xp("day", "2026-10-28", 99)).unwrap());
        assert!(db.insert_xp_event(&xp("day", "2026-10-29", 25)).unwrap());
        assert!(db.insert_xp_event(&xp("other", "2026-10-28", 5)).unwrap());
        assert_eq!(db.xp_total().unwrap(), 70);
        let events = db.xp_events().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].amount, 40);
        assert_eq!(events[0].mode, Mode::Diy);
    }

    #[test]
    fn xp_rejects_mode_none_yet() {
        let db = Db::open_in_memory().unwrap();
        let mut e = xp("day", "x", 1);
        e.mode = Mode::NoneYet;
        assert!(matches!(db.insert_xp_event(&e), Err(DbError::Invalid(_))));
    }

    #[test]
    fn achievement_unlock_is_sticky_progress_updates() {
        let db = Db::open_in_memory().unwrap();
        let progress = AchievementRow {
            id: "tokenMonk".into(),
            unlocked_at: None,
            retroactive: false,
            current: Some(2),
            target: Some(7),
            updated_at: 1,
        };
        db.upsert_achievement(&progress).unwrap();
        let unlocked = AchievementRow {
            unlocked_at: Some(50),
            retroactive: true,
            current: Some(7),
            updated_at: 50,
            ..progress.clone()
        };
        db.upsert_achievement(&unlocked).unwrap();
        // A later write must not move or clear an existing unlock.
        let later = AchievementRow {
            unlocked_at: Some(99),
            retroactive: false,
            current: Some(8),
            updated_at: 99,
            ..progress
        };
        db.upsert_achievement(&later).unwrap();
        let row = db.achievement("tokenMonk").unwrap().unwrap();
        assert_eq!(row.unlocked_at, Some(50));
        assert!(row.retroactive);
        assert_eq!(row.current, Some(8));
        assert_eq!(db.achievements().unwrap().len(), 1);
        assert_eq!(db.achievement("boo").unwrap(), None);
    }
}
