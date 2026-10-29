//! Local SQLite storage: connection, migrations, upserts and queries.
//!
//! One [`Db`] owns one connection. The ingest worker holds the only write
//! connection; readers open their own with [`Db::open`] (WAL keeps reads
//! from waiting on the writer). See docs/ARCHITECTURE.md ("Ingest", "Storage").

mod messages;
mod migrations;
mod queries;
mod source_files;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

pub use messages::UpsertStats;
pub use migrations::{latest_version, MIGRATIONS};
pub use queries::{DayTotals, LatestMessage, SessionSummary, TokenTotals};
pub use source_files::{plan_read, ReadPlan, SourceFile};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("database schema version {found} is newer than supported version {supported}")]
    NewerSchema { found: u32, supported: u32 },
}

pub type DbResult<T> = Result<T, DbError>;

pub struct Db {
    conn: Connection,
}

impl Db {
    /// Opens (creating if needed) the database at `path`, enables WAL and
    /// applies pending migrations.
    pub fn open(path: &Path) -> DbResult<Self> {
        Self::init(Connection::open(path)?)
    }

    /// In-memory database with all migrations applied (tests, scratch use).
    pub fn open_in_memory() -> DbResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    /// Like [`Db::open`], but a corrupt file or one from a newer schema is
    /// renamed aside (`<name>.bak-<unix ms>`) and a fresh database is created,
    /// to be rebuilt from the logs. Returns the backup path if one was made.
    pub fn open_or_rebuild(path: &Path) -> DbResult<(Self, Option<PathBuf>)> {
        match Self::open(path) {
            Ok(db) => Ok((db, None)),
            Err(e) if path.exists() && is_unusable(&e) => {
                let backup = move_aside(path)?;
                Ok((Self::open(path)?, Some(backup)))
            }
            Err(e) => Err(e),
        }
    }

    fn init(conn: Connection) -> DbResult<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        // `journal_mode` returns a row; in-memory databases report "memory".
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let mut db = Self { conn };
        migrations::migrate(&mut db.conn)?;
        Ok(db)
    }

    /// Current schema version (`PRAGMA user_version`).
    pub fn schema_version(&self) -> DbResult<u32> {
        migrations::user_version(&self.conn)
    }

    /// Underlying connection for modules that need raw SQL (settings, game).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Mutable connection, e.g. for transactions in other modules.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

/// Errors that mean the file itself cannot be used (as opposed to a busy
/// or unreadable one, which must not cost the user their database).
fn is_unusable(e: &DbError) -> bool {
    use rusqlite::ErrorCode::{DatabaseCorrupt, NotADatabase};
    match e {
        DbError::NewerSchema { .. } => true,
        DbError::Sqlite(rusqlite::Error::SqliteFailure(f, _)) => {
            matches!(f.code, DatabaseCorrupt | NotADatabase)
        }
        _ => false,
    }
}

/// Renames the database to `<name>.bak-<ms>` and moves its WAL side files
/// next to the backup, never next to the fresh database.
fn move_aside(path: &Path) -> std::io::Result<PathBuf> {
    let backup = backup_path(path);
    std::fs::rename(path, &backup)?;
    for suffix in ["-wal", "-shm"] {
        let side = with_suffix(path, suffix);
        if side.exists() {
            std::fs::rename(&side, with_suffix(&backup, suffix))?;
        }
    }
    Ok(backup)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn backup_path(path: &Path) -> PathBuf {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".bak-{ms}"));
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_wal_database_at_latest_version() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db")).unwrap();
        assert_eq!(db.schema_version().unwrap(), latest_version());
        let mode: String = db
            .conn()
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn newer_schema_is_rejected_and_rebuilt_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let db = Db::open(&path).unwrap();
            db.conn().pragma_update(None, "user_version", 99).unwrap();
        }
        assert!(matches!(
            Db::open(&path),
            Err(DbError::NewerSchema { found: 99, .. })
        ));
        let (db, backup) = Db::open_or_rebuild(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), latest_version());
        assert!(backup.unwrap().exists());
    }

    #[test]
    fn move_aside_keeps_wal_data_readable_in_the_backup() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("live.db");
        let path = dir.path().join("t.db");
        let writer = Db::open(&live).unwrap();
        writer
            .conn()
            .execute("INSERT INTO settings (key, value) VALUES ('k', '1')", [])
            .unwrap();
        // Snapshot main file and WAL while the writer is open, so the row
        // exists only in the WAL (no checkpoint has happened yet).
        std::fs::copy(&live, &path).unwrap();
        std::fs::copy(with_suffix(&live, "-wal"), with_suffix(&path, "-wal")).unwrap();
        drop(writer);

        let backup = move_aside(&path).unwrap();
        assert!(!path.exists());
        assert!(!with_suffix(&path, "-wal").exists());
        assert!(with_suffix(&backup, "-wal").exists());

        let old = Connection::open(&backup).unwrap();
        let v: String = old
            .query_row("SELECT value FROM settings WHERE key = 'k'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(v, "1");
    }

    #[test]
    fn corrupt_file_is_moved_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        std::fs::write(&path, vec![0x42u8; 4096]).unwrap();
        let (db, backup) = Db::open_or_rebuild(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), latest_version());
        assert!(backup.is_some());
    }

    #[test]
    fn parsed_chunks_ingest_incrementally() {
        let line = |id: &str, out: u32| {
            format!(
                r#"{{"type":"assistant","timestamp":"2026-10-28T21:14:03.512Z","sessionId":"s","message":{{"id":"{id}","model":"x","usage":{{"input_tokens":1,"output_tokens":{out}}}}}}}"#
            )
        };
        let mut data = format!("{}\n{}\n{}\n", line("m", 5), "junk", line("m", 9)).into_bytes();
        let partial = line("n", 1);
        data.extend_from_slice(&partial.as_bytes()[..30]);

        let mut db = Db::open_in_memory().unwrap();
        let first = crate::logs::parse_chunk(&data);
        let file = SourceFile {
            path: "/logs/a.jsonl".into(),
            size_bytes: data.len() as u64,
            offset_bytes: first.consumed as u64,
            mtime_ms: 1,
        };
        db.ingest_file_chunk(&file, &first.records, 1).unwrap();
        assert_eq!(db.message_count().unwrap(), 1);
        assert_eq!(
            plan_read(
                db.source_file(&file.path).unwrap().as_ref(),
                file.size_bytes + 10,
                2
            ),
            ReadPlan::Resume(first.consumed as u64)
        );
        let t = db.tokens_between(0, i64::MAX).unwrap();
        assert_eq!(t.output, 9);
    }

    #[test]
    fn reopen_keeps_data_and_does_not_remigrate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let db = Db::open(&path).unwrap();
            db.conn()
                .execute("INSERT INTO settings (key, value) VALUES ('k', '1')", [])
                .unwrap();
        }
        let db = Db::open(&path).unwrap();
        let n: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}
