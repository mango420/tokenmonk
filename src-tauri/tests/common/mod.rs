//! Shared helpers for the integration tests: fixture lookup and a minimal
//! file reader that follows the ingest rules of docs/ARCHITECTURE.md
//! ("Ingest") using only the public `logs` and `db` APIs.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use chrono::TimeZone;
use tokenmonk_lib::contracts::EpochMs;
use tokenmonk_lib::db::{plan_read, Db, ReadPlan, SourceFile, UpsertStats};
use tokenmonk_lib::logs::{parse_chunk, SkipCounts};

pub const S1: &str = "11111111-1111-4111-8111-111111111111";
pub const S2: &str = "22222222-2222-4222-8222-222222222222";
pub const S3: &str = "33333333-3333-4333-8333-333333333333";
pub const S4: &str = "44444444-4444-4444-8444-444444444444";

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The four files of the main set, in a fixed order:
/// session 1 main, session 2 (resumed), session 1 subagent, session 3.
pub fn main_set() -> Vec<PathBuf> {
    let demo = fixtures_dir().join("projects/-Users-example-projects-demo");
    let other = fixtures_dir().join("projects/-Users-example-projects-other");
    vec![
        demo.join(format!("{S1}.jsonl")),
        demo.join(format!("{S2}.jsonl")),
        demo.join(format!("{S1}/subagents/agent-a1b2c3d4e5f60718.jsonl")),
        other.join(format!("{S3}.jsonl")),
    ]
}

pub fn new_db(dir: &Path) -> Db {
    Db::open(&dir.join("tokenmonk-test.db")).expect("open temp db")
}

/// What one read of a file did.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadOutcome {
    pub plan: ReadPlan,
    pub stats: UpsertStats,
    pub skipped: SkipCounts,
    /// Offset stored after the read.
    pub offset: u64,
}

/// One pass over `path` as the ingest worker does it: plan from the stored
/// state, read from the planned offset, parse up to the last newline, store
/// records and new offset in one transaction. `mtime_ms` stands in for the
/// file's modification time so tests stay deterministic.
pub fn ingest_file(db: &mut Db, path: &Path, mtime_ms: EpochMs) -> ReadOutcome {
    let key = path.to_string_lossy().into_owned();
    let bytes = fs::read(path).expect("read fixture");
    let size = bytes.len() as u64;
    let stored = db.source_file(&key).expect("source_file");
    let plan = plan_read(stored.as_ref(), size, mtime_ms);
    let start = match plan {
        ReadPlan::Unchanged => {
            return ReadOutcome {
                plan,
                stats: UpsertStats::default(),
                skipped: SkipCounts::default(),
                offset: stored.map_or(0, |s| s.offset_bytes),
            };
        }
        ReadPlan::Resume(o) => o,
        ReadPlan::Restart => 0,
    };
    let chunk = parse_chunk(&bytes[start as usize..]);
    let offset = start + chunk.consumed as u64;
    let file = SourceFile {
        path: key,
        size_bytes: size,
        offset_bytes: offset,
        mtime_ms,
    };
    let stats = db
        .ingest_file_chunk(&file, &chunk.records, mtime_ms)
        .expect("ingest_file_chunk");
    ReadOutcome {
        plan,
        stats,
        skipped: chunk.skipped,
        offset,
    }
}

/// One stored message: id, request id, session, agent, sidechain flag, ts,
/// model, project path, input, output, cache creation, cache read.
pub type MessageRow = (
    String,
    Option<String>,
    String,
    Option<String>,
    bool,
    i64,
    String,
    String,
    i64,
    i64,
    i64,
    i64,
);

/// Every stored message ordered by id: the full observable table state.
pub fn snapshot(db: &Db) -> Vec<MessageRow> {
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT message_id, request_id, session_id, agent_id, is_sidechain, ts, model, \
             project_path, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens \
             FROM messages ORDER BY message_id",
        )
        .expect("prepare snapshot");
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
                r.get(10)?,
                r.get(11)?,
            ))
        })
        .expect("query snapshot");
    rows.collect::<Result<_, _>>().expect("rows")
}

/// Unix epoch ms of a UTC wall-clock time.
pub fn utc_ms(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> EpochMs {
    chrono::Utc
        .with_ymd_and_hms(y, mo, d, h, mi, s)
        .single()
        .expect("valid time")
        .timestamp_millis()
}
