//! Versioned, append-only migrations tracked in `PRAGMA user_version`.
//!
//! Never edit or reorder an entry once released; add a new one with the next
//! number (agreed with the architect, and keep `SCHEMA_VERSION` in the
//! contracts in step).

use rusqlite::Connection;

use super::{DbError, DbResult};
use crate::contracts::schema::SCHEMA_V1_SQL;

/// `(version, sql)` in ascending order without gaps, starting at 1.
pub const MIGRATIONS: &[(u32, &str)] = &[(1, SCHEMA_V1_SQL)];

pub fn latest_version() -> u32 {
    MIGRATIONS.last().map_or(0, |m| m.0)
}

pub(super) fn user_version(conn: &Connection) -> DbResult<u32> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    Ok(u32::try_from(v).unwrap_or(0))
}

/// Applies every migration above the stored version, each in its own
/// transaction together with the version bump.
pub(super) fn migrate(conn: &mut Connection) -> DbResult<()> {
    let current = user_version(conn)?;
    let supported = latest_version();
    if current > supported {
        return Err(DbError::NewerSchema {
            found: current,
            supported,
        });
    }
    for &(version, sql) in MIGRATIONS.iter().filter(|m| m.0 > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        // PRAGMA does not take bound parameters; `version` is a trusted constant.
        tx.execute_batch(&format!("PRAGMA user_version = {version}"))?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::schema::SCHEMA_VERSION;

    #[test]
    fn migrations_are_contiguous_and_match_contract() {
        for (i, (v, _)) in MIGRATIONS.iter().enumerate() {
            assert_eq!(*v as usize, i + 1);
        }
        assert_eq!(latest_version(), SCHEMA_VERSION);
    }

    #[test]
    fn migrate_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), latest_version());
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN \
                 ('source_files','messages','limit_snapshots','settings','xp_events','achievements','app_state')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 7);
    }

    #[test]
    fn failed_migration_rolls_back() {
        let mut conn = Connection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        assert!(tx
            .execute_batch("CREATE TABLE a (x); CREATE TABLE a (x);")
            .is_err());
        drop(tx);
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
        assert_eq!(user_version(&conn).unwrap(), 0);
    }
}
