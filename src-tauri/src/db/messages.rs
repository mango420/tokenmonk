//! Order-independent upsert of usage records (docs/LOG-FORMAT.md,
//! "Deduplication"; docs/ARCHITECTURE.md, "Ingest").

use rusqlite::{params, Connection};

use super::source_files::{upsert_source_file, SourceFile};
use super::{Db, DbResult};
use crate::contracts::EpochMs;
use crate::logs::UsageRecord;

/// Outcome of an upsert batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UpsertStats {
    /// Records offered.
    pub records: usize,
    /// Message ids that did not exist before.
    pub inserted: usize,
}

fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

const UPSERT_SQL: &str = "\
INSERT INTO messages (message_id, request_id, session_id, agent_id, is_sidechain, ts, model, \
    project_path, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens) \
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) \
ON CONFLICT(message_id) DO UPDATE SET \
    request_id = COALESCE(request_id, excluded.request_id), \
    ts = MIN(ts, excluded.ts), \
    input_tokens = MAX(input_tokens, excluded.input_tokens), \
    output_tokens = MAX(output_tokens, excluded.output_tokens), \
    cache_creation_tokens = MAX(cache_creation_tokens, excluded.cache_creation_tokens), \
    cache_read_tokens = MAX(cache_read_tokens, excluded.cache_read_tokens)";

fn upsert_all(conn: &Connection, records: &[UsageRecord]) -> DbResult<UpsertStats> {
    let mut stmt = conn.prepare_cached(UPSERT_SQL)?;
    let mut exists = conn.prepare_cached("SELECT 1 FROM messages WHERE message_id = ?1")?;
    let mut stats = UpsertStats::default();
    for r in records {
        if !exists.exists([&r.message_id])? {
            stats.inserted += 1;
        }
        stmt.execute(params![
            r.message_id,
            r.request_id,
            r.session_id,
            r.agent_id,
            r.is_sidechain,
            r.ts,
            r.model,
            r.project_path,
            to_i64(r.input_tokens),
            to_i64(r.output_tokens),
            to_i64(r.cache_creation_tokens),
            to_i64(r.cache_read_tokens),
        ])?;
        stats.records += 1;
    }
    Ok(stats)
}

impl Db {
    /// Upserts records in one transaction. Safe to call with repeats and in
    /// any order: `ts` keeps the minimum, token columns the maximum.
    pub fn upsert_messages(&mut self, records: &[UsageRecord]) -> DbResult<UpsertStats> {
        let tx = self.conn_mut().transaction()?;
        let stats = upsert_all(&tx, records)?;
        tx.commit()?;
        Ok(stats)
    }

    /// Upserts the records of one file read and stores the file's new read
    /// state in the same transaction, so offset and data never diverge.
    pub fn ingest_file_chunk(
        &mut self,
        file: &SourceFile,
        records: &[UsageRecord],
        now_ms: EpochMs,
    ) -> DbResult<UpsertStats> {
        let tx = self.conn_mut().transaction()?;
        let stats = upsert_all(&tx, records)?;
        upsert_source_file(&tx, file, now_ms)?;
        tx.commit()?;
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn rec(id: &str, ts: EpochMs, out: u64) -> UsageRecord {
        UsageRecord {
            message_id: id.into(),
            request_id: Some("req_1".into()),
            session_id: "sess-1".into(),
            agent_id: None,
            is_sidechain: false,
            ts,
            model: "model-x".into(),
            project_path: "/work/demo-app".into(),
            input_tokens: 5,
            output_tokens: out,
            cache_creation_tokens: 100,
            cache_read_tokens: 1000,
        }
    }

    fn row(db: &Db, id: &str) -> (i64, i64, i64, i64, i64) {
        db.conn()
            .query_row(
                "SELECT ts, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens \
                 FROM messages WHERE message_id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap()
    }

    #[test]
    fn repeat_takes_min_ts_and_max_tokens() {
        let mut db = Db::open_in_memory().unwrap();
        let stats = db
            .upsert_messages(&[rec("m", 2000, 10), rec("m", 1000, 30), rec("m", 3000, 20)])
            .unwrap();
        assert_eq!(
            stats,
            UpsertStats {
                records: 3,
                inserted: 1
            }
        );
        assert_eq!(row(&db, "m"), (1000, 5, 30, 100, 1000));
    }

    #[test]
    fn result_is_independent_of_order_and_batching() {
        let a = [rec("m", 3000, 5), rec("m", 1000, 50), rec("n", 500, 1)];
        let mut fwd = Db::open_in_memory().unwrap();
        fwd.upsert_messages(&a).unwrap();
        let mut rev = Db::open_in_memory().unwrap();
        for r in a.iter().rev() {
            rev.upsert_messages(std::slice::from_ref(r)).unwrap();
        }
        for id in ["m", "n"] {
            assert_eq!(row(&fwd, id), row(&rev, id));
        }
        assert_eq!(row(&fwd, "m"), (1000, 5, 50, 100, 1000));
    }

    #[test]
    fn stored_identity_columns_are_kept() {
        let mut db = Db::open_in_memory().unwrap();
        let mut first = rec("m", 1000, 1);
        first.request_id = None;
        db.upsert_messages(&[first]).unwrap();
        let mut second = rec("m", 2000, 2);
        second.session_id = "other".into();
        db.upsert_messages(&[second]).unwrap();
        let (sess, req): (String, Option<String>) = db
            .conn()
            .query_row(
                "SELECT session_id, request_id FROM messages WHERE message_id = 'm'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(sess, "sess-1");
        assert_eq!(req.as_deref(), Some("req_1"));
    }

    #[test]
    fn ingest_stores_offset_with_messages() {
        let mut db = Db::open_in_memory().unwrap();
        let file = SourceFile {
            path: "/logs/a.jsonl".into(),
            size_bytes: 200,
            offset_bytes: 180,
            mtime_ms: 7,
        };
        let stats = db
            .ingest_file_chunk(&file, &[rec("m", 1, 1), rec("n", 2, 2)], 9)
            .unwrap();
        assert_eq!(stats.inserted, 2);
        assert_eq!(db.source_file("/logs/a.jsonl").unwrap(), Some(file));
    }

    #[test]
    fn huge_token_counts_do_not_wrap() {
        let mut db = Db::open_in_memory().unwrap();
        let mut r = rec("m", 1, 1);
        r.output_tokens = u64::MAX;
        db.upsert_messages(&[r]).unwrap();
        assert_eq!(row(&db, "m").2, i64::MAX);
    }
}
