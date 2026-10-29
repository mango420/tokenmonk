//! Per-file read bookkeeping for incremental ingest.

use rusqlite::{params, OptionalExtension};

use super::{Db, DbResult};
use crate::contracts::EpochMs;

/// Read progress of one log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    /// File size at the last read.
    pub size_bytes: u64,
    /// Byte after the last complete line read.
    pub offset_bytes: u64,
    /// Modification time at the last read.
    pub mtime_ms: EpochMs,
}

/// What to do with a file given its stored state and its current metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadPlan {
    /// Nothing new.
    Unchanged,
    /// Read from this byte offset.
    Resume(u64),
    /// New, shrunk or replaced file: read from 0 (dedup makes it harmless).
    Restart,
}

/// Decides how to read a file (docs/LOG-FORMAT.md, "Partial and malformed
/// lines"). A partial last line leaves `size > offset`; it is only retried
/// when size or mtime changed.
pub fn plan_read(stored: Option<&SourceFile>, size: u64, mtime_ms: EpochMs) -> ReadPlan {
    let Some(s) = stored else {
        return ReadPlan::Restart;
    };
    // Log files are append-only: any shrink or older mtime means replacement.
    if size < s.size_bytes || size < s.offset_bytes || mtime_ms < s.mtime_ms {
        ReadPlan::Restart
    } else if size == s.size_bytes && mtime_ms == s.mtime_ms {
        ReadPlan::Unchanged
    } else if size > s.offset_bytes {
        ReadPlan::Resume(s.offset_bytes)
    } else {
        ReadPlan::Unchanged
    }
}

fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

impl Db {
    pub fn source_file(&self, path: &str) -> DbResult<Option<SourceFile>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT path, size_bytes, offset_bytes, mtime_ms FROM source_files WHERE path = ?1",
                [path],
                row_to_source_file,
            )
            .optional()?)
    }

    pub fn list_source_files(&self) -> DbResult<Vec<SourceFile>> {
        let mut stmt = self.conn().prepare(
            "SELECT path, size_bytes, offset_bytes, mtime_ms FROM source_files ORDER BY path",
        )?;
        let rows = stmt.query_map([], row_to_source_file)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Stores progress for a file without touching messages (for example a
    /// file with no new complete lines).
    pub fn set_source_file(&self, file: &SourceFile, now_ms: EpochMs) -> DbResult<()> {
        upsert_source_file(self.conn(), file, now_ms)
    }

    /// Forgets a file (deleted from disk).
    pub fn remove_source_file(&self, path: &str) -> DbResult<()> {
        self.conn()
            .execute("DELETE FROM source_files WHERE path = ?1", [path])?;
        Ok(())
    }
}

pub(super) fn upsert_source_file(
    conn: &rusqlite::Connection,
    file: &SourceFile,
    now_ms: EpochMs,
) -> DbResult<()> {
    conn.execute(
        "INSERT INTO source_files (path, size_bytes, offset_bytes, mtime_ms, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(path) DO UPDATE SET size_bytes = excluded.size_bytes, \
         offset_bytes = excluded.offset_bytes, mtime_ms = excluded.mtime_ms, \
         updated_at = excluded.updated_at",
        params![
            file.path,
            to_i64(file.size_bytes),
            to_i64(file.offset_bytes),
            file.mtime_ms,
            now_ms
        ],
    )?;
    Ok(())
}

fn row_to_source_file(r: &rusqlite::Row<'_>) -> rusqlite::Result<SourceFile> {
    Ok(SourceFile {
        path: r.get(0)?,
        size_bytes: u64::try_from(r.get::<_, i64>(1)?).unwrap_or(0),
        offset_bytes: u64::try_from(r.get::<_, i64>(2)?).unwrap_or(0),
        mtime_ms: r.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(size: u64, offset: u64, mtime: EpochMs) -> SourceFile {
        SourceFile {
            path: "/logs/a.jsonl".into(),
            size_bytes: size,
            offset_bytes: offset,
            mtime_ms: mtime,
        }
    }

    #[test]
    fn plan_read_cases() {
        assert_eq!(plan_read(None, 10, 5), ReadPlan::Restart);
        let s = file(100, 100, 1000);
        assert_eq!(plan_read(Some(&s), 100, 1000), ReadPlan::Unchanged);
        assert_eq!(plan_read(Some(&s), 150, 1100), ReadPlan::Resume(100));
        assert_eq!(plan_read(Some(&s), 50, 1100), ReadPlan::Restart);
        assert_eq!(plan_read(Some(&s), 150, 900), ReadPlan::Restart);
        // Touched without growing.
        assert_eq!(plan_read(Some(&s), 100, 1100), ReadPlan::Unchanged);
        // Partial tail pending (size > offset) but nothing changed since.
        let p = file(120, 100, 1000);
        assert_eq!(plan_read(Some(&p), 120, 1000), ReadPlan::Unchanged);
        assert_eq!(plan_read(Some(&p), 140, 1001), ReadPlan::Resume(100));
        // Shrunk, but still past the offset: replaced all the same.
        assert_eq!(plan_read(Some(&p), 110, 1001), ReadPlan::Restart);
    }

    #[test]
    fn bookkeeping_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.source_file("/logs/a.jsonl").unwrap(), None);
        db.set_source_file(&file(10, 8, 1), 1).unwrap();
        db.set_source_file(&file(30, 30, 2), 2).unwrap();
        assert_eq!(
            db.source_file("/logs/a.jsonl").unwrap(),
            Some(file(30, 30, 2))
        );
        assert_eq!(db.list_source_files().unwrap().len(), 1);
        db.remove_source_file("/logs/a.jsonl").unwrap();
        assert!(db.list_source_files().unwrap().is_empty());
    }
}
