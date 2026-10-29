//! Incremental reads: partial last line, growth, truncation and replacement
//! (docs/LOG-FORMAT.md, "Partial and malformed lines"; docs/ARCHITECTURE.md,
//! "Ingest"). The `mtime` argument of `ingest_file` is the simulated file
//! modification time in epoch ms.

mod common;

use std::fs;
use std::path::Path;

use common::*;
use tokenmonk_lib::db::{Db, ReadPlan, TokenTotals};

fn partial_dir() -> std::path::PathBuf {
    fixtures_dir().join("partial")
}

fn totals(db: &Db) -> TokenTotals {
    db.tokens_between(0, i64::MAX).unwrap()
}

fn append(path: &Path, bytes: &[u8]) {
    use std::io::Write;
    let mut f = fs::OpenOptions::new().append(true).open(path).unwrap();
    f.write_all(bytes).unwrap();
}

/// Final state of the partial fixture once completed:
/// P1 (3 in, 80 out, 0 cc, 1000 cr), P2 (3, 220, 500, 2000), P3 (2, 60, 0, 3000)
/// input 3+3+2 = 8, output 80+220+60 = 360, cache creation 500, cache read 6000.
const PARTIAL_TOTAL: TokenTotals = TokenTotals {
    messages: 3,
    input: 8,
    output: 360,
    cache_creation: 500,
    cache_read: 6000,
};

#[test]
fn partial_last_line_is_not_consumed_until_completed() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("tail.jsonl");
    let head = fs::read(partial_dir().join("tail.jsonl")).unwrap();
    let rest = fs::read(partial_dir().join("tail.rest.txt")).unwrap();
    fs::write(&log, &head).unwrap();
    assert_ne!(head.last(), Some(&b'\n'), "fixture ends in a partial line");
    let complete_len = head.iter().rposition(|&b| b == b'\n').unwrap() as u64 + 1;

    let mut db = new_db(dir.path());

    // First pass: P1 and the first block of P2 are stored, the cut-off line
    // is left alone and the offset stops after the last newline.
    let first = ingest_file(&mut db, &log, 100);
    assert_eq!(first.plan, ReadPlan::Restart);
    assert_eq!(first.stats.records, 2);
    assert_eq!(first.stats.inserted, 2);
    assert_eq!(first.skipped.not_assistant, 1);
    assert_eq!(
        first.skipped.malformed, 0,
        "a partial line is not malformed"
    );
    assert_eq!(first.offset, complete_len);
    assert!(first.offset < head.len() as u64);
    let stored = db.source_file(&log.to_string_lossy()).unwrap().unwrap();
    assert_eq!(
        (stored.size_bytes, stored.offset_bytes),
        (head.len() as u64, complete_len)
    );
    let p2_out = |db: &Db| {
        snapshot(db)
            .iter()
            .find(|r| r.0 == format!("msg_{:024}", 21))
            .unwrap()
            .9
    };
    assert_eq!(p2_out(&db), 10, "only the first block of P2 so far");

    // Nothing changed on disk: nothing is read.
    assert_eq!(ingest_file(&mut db, &log, 100).plan, ReadPlan::Unchanged);

    // Touched without growing: retried from the stored offset, still partial.
    let touched = ingest_file(&mut db, &log, 150);
    assert_eq!(touched.plan, ReadPlan::Resume(complete_len));
    assert_eq!(touched.stats.records, 0);
    assert_eq!(touched.offset, complete_len);

    // The writer finishes the line and adds P3.
    append(&log, &rest);
    let done = ingest_file(&mut db, &log, 200);
    assert_eq!(done.plan, ReadPlan::Resume(complete_len));
    assert_eq!(done.stats.records, 2, "completed P2 line and P3");
    assert_eq!(done.stats.inserted, 1, "P2 existed, P3 is new");
    assert_eq!(done.offset, (head.len() + rest.len()) as u64);
    assert_eq!(p2_out(&db), 220, "final output count of P2");
    assert_eq!(totals(&db), PARTIAL_TOTAL);

    // A cold start on the finished file gives the same table.
    let dir2 = tempfile::tempdir().unwrap();
    let mut cold = new_db(dir2.path());
    ingest_file(&mut cold, &log, 200);
    assert_eq!(snapshot(&cold), snapshot(&db));
}

#[test]
fn complete_json_without_trailing_newline_waits_for_the_newline() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("s3.jsonl");
    let full = fs::read(main_set()[3].clone()).unwrap();
    assert_eq!(full.last(), Some(&b'\n'));
    fs::write(&log, &full[..full.len() - 1]).unwrap();

    let mut db = new_db(dir.path());
    // The last line (M4) is valid JSON, but without its newline it may still
    // be growing, so it is not consumed.
    let first = ingest_file(&mut db, &log, 100);
    assert_eq!(first.stats.records, 4, "M1, M2 and the two lines of M3");
    assert_eq!(first.stats.inserted, 3);
    assert_eq!(totals(&db).messages, 3);

    append(&log, b"\n");
    let second = ingest_file(&mut db, &log, 101);
    assert_eq!(second.plan, ReadPlan::Resume(first.offset));
    assert_eq!(second.stats.inserted, 1);
    // M1..M4: input 2+2+4+4 = 12, output 100+50+700+300 = 1150,
    // cache creation 1000, cache read 200000+210000+250000+260000 = 920000.
    assert_eq!(
        totals(&db),
        TokenTotals {
            messages: 4,
            input: 12,
            output: 1150,
            cache_creation: 1000,
            cache_read: 920_000,
        }
    );
}

#[test]
fn grown_file_reads_only_the_new_lines() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("s2.jsonl");
    let full = fs::read(main_set()[1].clone()).unwrap();
    // Split after the 5th line (user copy, A, A, B, C): the copied history.
    let cut = full
        .iter()
        .enumerate()
        .filter(|(_, &b)| b == b'\n')
        .nth(4)
        .map(|(i, _)| i + 1)
        .unwrap();
    fs::write(&log, &full[..cut]).unwrap();

    let mut db = new_db(dir.path());
    let first = ingest_file(&mut db, &log, 100);
    assert_eq!(first.stats.records, 4);
    assert_eq!(first.stats.inserted, 3, "A, B, C");
    assert_eq!(first.offset, cut as u64);

    append(&log, &full[cut..]);
    let second = ingest_file(&mut db, &log, 200);
    assert_eq!(second.plan, ReadPlan::Resume(cut as u64));
    assert_eq!(second.stats.records, 3, "D twice and E, nothing re-read");
    assert_eq!(second.stats.inserted, 2);
    assert_eq!(second.offset, full.len() as u64);
    assert_eq!(totals(&db).messages, 5);
}

#[test]
fn truncated_or_replaced_file_restarts_from_zero() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("replaced.jsonl");
    let s3 = fs::read(main_set()[3].clone()).unwrap();
    fs::write(&log, &s3).unwrap();

    let mut db = new_db(dir.path());
    let first = ingest_file(&mut db, &log, 1_000);
    assert_eq!(first.stats.inserted, 4);
    let before = snapshot(&db);

    // Shrunk: only the first three lines (user, M1, M2) remain.
    let cut = s3
        .iter()
        .enumerate()
        .filter(|(_, &b)| b == b'\n')
        .nth(2)
        .map(|(i, _)| i + 1)
        .unwrap();
    fs::write(&log, &s3[..cut]).unwrap();
    let shrunk = ingest_file(&mut db, &log, 2_000);
    assert_eq!(shrunk.plan, ReadPlan::Restart);
    assert_eq!(shrunk.stats.records, 2, "re-read from byte 0: M1 and M2");
    assert_eq!(shrunk.stats.inserted, 0, "dedup: nothing new");
    assert_eq!(shrunk.offset, cut as u64);
    assert_eq!(snapshot(&db), before, "rows already stored are kept");

    // Replaced by a bigger file whose modification time went backwards.
    let s1 = fs::read(main_set()[0].clone()).unwrap();
    assert!(s1.len() > cut);
    fs::write(&log, &s1).unwrap();
    let replaced = ingest_file(&mut db, &log, 1_500);
    assert_eq!(replaced.plan, ReadPlan::Restart);
    assert_eq!(replaced.offset, s1.len() as u64);
    assert_eq!(replaced.stats.records, 5, "A x3, B, C");
    assert_eq!(replaced.stats.inserted, 3);
    assert_eq!(totals(&db).messages, 7);

    // Forgetting the file (deleted from disk) makes the next sight a restart.
    db.remove_source_file(&log.to_string_lossy()).unwrap();
    assert!(db.source_file(&log.to_string_lossy()).unwrap().is_none());
    let again = ingest_file(&mut db, &log, 1_500);
    assert_eq!(again.plan, ReadPlan::Restart);
    assert_eq!(again.stats.inserted, 0);
}
