//! Incremental read of one log file into the database.
//!
//! Uses the stored byte offset (`plan_read`), parses complete lines only
//! (`parse_chunk`) and stores records and the new offset in one transaction
//! (`ingest_file_chunk`). The file is opened read only and never modified.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::contracts::EpochMs;
use crate::db::{plan_read, Db, ReadPlan, SourceFile};
use crate::logs::{complete_prefix_len, parse_chunk, UsageRecord};

/// Bytes read per step. A single line longer than this still works: the
/// buffer grows until it holds a newline.
pub const CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// What happened to a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOutcome {
    /// Nothing new since the stored state.
    Unchanged,
    /// New bytes were read.
    Ingested {
        /// Records parsed (repeats of stored messages included).
        records: usize,
        /// Messages that were not stored before.
        inserted: usize,
    },
    /// The file is gone; its read state was dropped (stored messages stay).
    Removed,
    /// The file could not be read or stored; it is retried on the next scan.
    Failed(String),
}

impl FileOutcome {
    /// Whether the database content or bookkeeping may have changed.
    pub fn changed(&self) -> bool {
        matches!(self, FileOutcome::Ingested { .. } | FileOutcome::Removed)
    }
}

fn mtime_ms(meta: &fs::Metadata) -> EpochMs {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Reads from `offset` in steps of [`CHUNK_BYTES`] until the buffer holds a
/// newline or `size` is reached.
fn read_complete_lines(file: &mut File, offset: u64, size: u64) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    let mut at = offset;
    while at < size {
        let step = CHUNK_BYTES.min(size - at);
        let start = buf.len();
        buf.resize(start + usize::try_from(step).unwrap_or(usize::MAX), 0);
        file.read_exact(&mut buf[start..])?;
        at += step;
        if complete_prefix_len(&buf) > 0 {
            break;
        }
    }
    Ok(buf)
}

/// Cheap pre-check for progress counting: whether [`ingest_file`] would read
/// anything from `path`.
pub fn needs_work(db: &Db, path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    match db.source_file(&path.to_string_lossy()) {
        Ok(stored) => {
            plan_read(stored.as_ref(), meta.len(), mtime_ms(&meta)) != ReadPlan::Unchanged
        }
        Err(_) => true,
    }
}

/// Brings the stored state of `path` up to date.
///
/// `on_chunk` is called after each committed chunk with the parsed records
/// (they may repeat messages that were stored before).
pub fn ingest_file(
    db: &mut Db,
    path: &Path,
    now: EpochMs,
    on_chunk: &mut dyn FnMut(&mut Db, &[UsageRecord]),
) -> FileOutcome {
    let key = path.to_string_lossy().into_owned();
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return match db.source_file(&key) {
                Ok(Some(_)) => match db.remove_source_file(&key) {
                    Ok(()) => FileOutcome::Removed,
                    Err(e) => FileOutcome::Failed(e.to_string()),
                },
                Ok(None) => FileOutcome::Unchanged,
                Err(e) => FileOutcome::Failed(e.to_string()),
            };
        }
        Err(e) => return FileOutcome::Failed(e.to_string()),
    };
    if !meta.is_file() {
        return FileOutcome::Unchanged;
    }
    let size = meta.len();
    let mtime = mtime_ms(&meta);

    let stored = match db.source_file(&key) {
        Ok(s) => s,
        Err(e) => return FileOutcome::Failed(e.to_string()),
    };
    let mut offset = match plan_read(stored.as_ref(), size, mtime) {
        ReadPlan::Unchanged => return FileOutcome::Unchanged,
        ReadPlan::Resume(o) => o,
        ReadPlan::Restart => 0,
    };

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return FileOutcome::Failed(e.to_string()),
    };

    let (mut records, mut inserted) = (0usize, 0usize);
    let mut wrote_state = false;
    while offset < size {
        let buf = match read_complete_lines(&mut file, offset, size) {
            Ok(b) => b,
            Err(e) => return FileOutcome::Failed(e.to_string()),
        };
        let chunk = parse_chunk(&buf);
        let new_offset = offset + chunk.consumed as u64;
        let reached_end = offset + buf.len() as u64 >= size;
        // While more of the file is left to read, the stored size must stay
        // behind the real size so an interrupted import resumes.
        let state = SourceFile {
            path: key.clone(),
            size_bytes: if reached_end { size } else { new_offset },
            offset_bytes: new_offset,
            mtime_ms: mtime,
        };
        match db.ingest_file_chunk(&state, &chunk.records, now) {
            Ok(stats) => {
                records += chunk.records.len();
                inserted += stats.inserted;
                wrote_state = true;
            }
            Err(e) => return FileOutcome::Failed(e.to_string()),
        }
        on_chunk(db, &chunk.records);
        if chunk.consumed == 0 {
            // Only a partial last line is left; retried when the file changes.
            break;
        }
        offset = new_offset;
    }

    if !wrote_state {
        // Empty file, or new bytes that never completed a line: remember the
        // size so the file is not re-read until it changes again.
        let state = SourceFile {
            path: key,
            size_bytes: size,
            offset_bytes: offset.min(size),
            mtime_ms: mtime,
        };
        if let Err(e) = db.set_source_file(&state, now) {
            return FileOutcome::Failed(e.to_string());
        }
    }
    FileOutcome::Ingested { records, inserted }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    pub(crate) fn line(id: &str, session: &str, ts_ms: i64, out: u64) -> String {
        let ts = chrono::DateTime::from_timestamp_millis(ts_ms)
            .unwrap()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ");
        format!(
            r#"{{"type":"assistant","timestamp":"{ts}","sessionId":"{session}","cwd":"/work/demo-app","requestId":"req_{id}","message":{{"id":"{id}","model":"model-x","usage":{{"input_tokens":1,"output_tokens":{out},"cache_creation_input_tokens":0,"cache_read_input_tokens":7}}}}}}"#
        )
    }

    fn nop(_: &mut Db, _: &[UsageRecord]) {}

    fn append(path: &Path, text: &str) {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    #[test]
    fn reads_new_lines_incrementally_and_retries_partial_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jsonl");
        let mut db = Db::open_in_memory().unwrap();

        append(&path, &format!("{}\njunk\n", line("m1", "s", 1_000, 10)));
        let half = line("m2", "s", 2_000, 20);
        append(&path, &half[..40]);
        assert_eq!(
            ingest_file(&mut db, &path, 5, &mut nop),
            FileOutcome::Ingested {
                records: 1,
                inserted: 1
            }
        );
        assert_eq!(db.message_count().unwrap(), 1);
        // Unchanged file: no work.
        assert_eq!(
            ingest_file(&mut db, &path, 6, &mut nop),
            FileOutcome::Unchanged
        );

        append(&path, &format!("{}\n", &half[40..]));
        assert_eq!(
            ingest_file(&mut db, &path, 7, &mut nop),
            FileOutcome::Ingested {
                records: 1,
                inserted: 1
            }
        );
        assert_eq!(db.message_count().unwrap(), 2);
        let t = db.tokens_between(0, i64::MAX).unwrap();
        assert_eq!((t.output, t.cache_read), (30, 14));
    }

    #[test]
    fn replaced_file_is_reread_without_double_counting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jsonl");
        let mut db = Db::open_in_memory().unwrap();
        append(
            &path,
            &format!(
                "{}\n{}\n",
                line("m1", "s", 1_000, 10),
                line("m2", "s", 2_000, 20)
            ),
        );
        ingest_file(&mut db, &path, 1, &mut nop);
        // Shorter replacement with one repeated message.
        fs::write(&path, format!("{}\n", line("m1", "s", 1_000, 10))).unwrap();
        let out = ingest_file(&mut db, &path, 2, &mut nop);
        assert_eq!(
            out,
            FileOutcome::Ingested {
                records: 1,
                inserted: 0
            }
        );
        assert_eq!(db.message_count().unwrap(), 2);
        assert_eq!(db.tokens_between(0, i64::MAX).unwrap().output, 30);
    }

    #[test]
    fn deleted_file_drops_its_read_state_but_keeps_messages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jsonl");
        let mut db = Db::open_in_memory().unwrap();
        append(&path, &format!("{}\n", line("m1", "s", 1_000, 10)));
        ingest_file(&mut db, &path, 1, &mut nop);
        fs::remove_file(&path).unwrap();
        assert_eq!(
            ingest_file(&mut db, &path, 2, &mut nop),
            FileOutcome::Removed
        );
        assert!(db.list_source_files().unwrap().is_empty());
        assert_eq!(db.message_count().unwrap(), 1);
        assert_eq!(
            ingest_file(&mut db, &path, 3, &mut nop),
            FileOutcome::Unchanged
        );
    }

    #[test]
    fn empty_file_is_remembered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jsonl");
        fs::write(&path, "").unwrap();
        let mut db = Db::open_in_memory().unwrap();
        assert!(ingest_file(&mut db, &path, 1, &mut nop).changed());
        assert_eq!(
            ingest_file(&mut db, &path, 2, &mut nop),
            FileOutcome::Unchanged
        );
    }

    #[test]
    fn callback_sees_each_chunk_and_the_file_is_never_modified() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jsonl");
        let text = format!("{}\n", line("m1", "s", 1_000, 10));
        fs::write(&path, &text).unwrap();
        let mut db = Db::open_in_memory().unwrap();
        let mut seen = Vec::new();
        ingest_file(&mut db, &path, 1, &mut |_, recs| {
            seen.extend(recs.iter().map(|r| r.message_id.clone()));
        });
        assert_eq!(seen, ["m1"]);
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn a_line_longer_than_one_chunk_is_read_whole() {
        // Exercise the growing buffer with a small artificial line: the
        // helper itself is size independent, so test it directly.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jsonl");
        let big = "x".repeat((CHUNK_BYTES as usize) + 10);
        fs::write(&path, format!("{big}\n{}\n", line("m1", "s", 1_000, 10))).unwrap();
        let mut db = Db::open_in_memory().unwrap();
        let out = ingest_file(&mut db, &path, 1, &mut nop);
        assert_eq!(
            out,
            FileOutcome::Ingested {
                records: 1,
                inserted: 1
            }
        );
        assert_eq!(db.message_count().unwrap(), 1);
    }
}
