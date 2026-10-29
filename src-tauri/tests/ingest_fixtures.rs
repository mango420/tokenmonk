//! Parse and ingest the synthetic fixtures (docs/LOG-FORMAT.md). All
//! expected numbers are computed by hand from the fixture files; the
//! message key below maps `msg_<24 digits>` ids to letters.
//!
//! | key | id suffix | where                                  | final in/out/cc/cr           |
//! |-----|-----------|----------------------------------------|------------------------------|
//! | A   | 1         | S1 main (3 lines), S2 copy (2 lines)   | 6 / 380 / 1500 / 40000       |
//! | B   | 2         | S1 main, S2 copy                       | 8 / 220 / 300 / 41500        |
//! | C   | 3         | S1 main, S2 copy                       | 4 / 90 / 0 / 42000           |
//! | D   | 4         | S2 only (2 lines)                      | 5 / 500 / 2000 / 45000       |
//! | E   | 5         | S2 only                                | 3 / 60 / 0 / 47000           |
//! | S1x | 6         | subagent of S1 (2 lines)               | 10 / 310 / 5000 / 0          |
//! | S2x | 7         | subagent of S1                         | 3 / 75 / 0 / 5000            |
//! | M1  | 8         | S3, 2026-10-27T21:59:59.999Z           | 2 / 100 / 0 / 200000         |
//! | M2  | 9         | S3, 2026-10-27T22:00:00.000Z           | 2 / 50 / 0 / 210000          |
//! | M3  | 10        | S3, 2026-10-28T08:15:00Z (2 lines)     | 4 / 700 / 1000 / 250000     |
//! | M4  | 11        | S3, 2026-10-28T11:30:00Z               | 4 / 300 / 0 / 260000         |

mod common;

use std::fs;
use std::path::PathBuf;

use common::*;
use tokenmonk_lib::db::{Db, ReadPlan, TokenTotals};
use tokenmonk_lib::logs::{parse_chunk, SkipCounts};

fn mid(k: u32) -> String {
    format!("msg_{k:024}")
}

fn all_permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut out = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let head = rest.remove(i);
        for mut tail in all_permutations(&rest) {
            tail.insert(0, head.clone());
            out.push(tail);
        }
    }
    out
}

/// Totals of the 11 unique messages.
/// input  6+8+4+5+3+10+3+2+2+4+4 = 51
/// output 380+220+90+500+60+310+75+100+50+700+300 = 2785
/// cache creation 1500+300+0+2000+0+5000+0+0+0+1000+0 = 9800
/// cache read 40000+41500+42000+45000+47000+0+5000+200000+210000+250000+260000 = 1140500
const TOTAL: TokenTotals = TokenTotals {
    messages: 11,
    input: 51,
    output: 2785,
    cache_creation: 9800,
    cache_read: 1_140_500,
};

/// Ingests the files one after another into a fresh temp-file database.
/// The temp dir is returned so it outlives the connection.
fn ingest_in_order(order: &[PathBuf]) -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let mut db = new_db(dir.path());
    for (i, p) in order.iter().enumerate() {
        ingest_file(&mut db, p, 1_000 + i as i64);
    }
    (dir, db)
}

fn totals(db: &Db) -> TokenTotals {
    db.tokens_between(0, i64::MAX).unwrap()
}

#[test]
fn every_fixture_parses_with_expected_counts() {
    // (file index in main_set, records before dedup, skipped by reason)
    let expected: [(usize, usize, SkipCounts); 4] = [
        // S1 main: A x3, B, C = 5 records. Skipped: 9 not-assistant lines
        // (2 user, 2 system, attachment, api-request-blob, queue-operation,
        // last-prompt, cost-state), 2 synthetic, 1 malformed, 1 without
        // usage, 1 without message.id.
        (
            0,
            5,
            SkipCounts {
                malformed: 1,
                not_assistant: 9,
                synthetic: 2,
                missing_usage: 1,
                missing_field: 1,
            },
        ),
        // S2: A x2 (copy), B, C, D x2, E = 7 records; 2 user lines skipped.
        (
            1,
            7,
            SkipCounts {
                not_assistant: 2,
                ..SkipCounts::default()
            },
        ),
        // Subagent: S1x x2, S2x = 3 records; 2 user lines skipped.
        (
            2,
            3,
            SkipCounts {
                not_assistant: 2,
                ..SkipCounts::default()
            },
        ),
        // S3: M1, M2, M3 x2, M4 = 5 records; 1 user and 1 system line skipped.
        (
            3,
            5,
            SkipCounts {
                not_assistant: 2,
                ..SkipCounts::default()
            },
        ),
    ];
    let files = main_set();
    for (idx, records, skipped) in expected {
        let bytes = fs::read(&files[idx]).unwrap();
        let chunk = parse_chunk(&bytes);
        let name = files[idx]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(chunk.records.len(), records, "records in {name}");
        assert_eq!(chunk.skipped, skipped, "skipped in {name}");
        assert_eq!(
            chunk.consumed,
            bytes.len(),
            "file ends with a newline: {name}"
        );
    }
}

#[test]
fn parsed_fields_match_the_log_lines() {
    let files = main_set();

    // First line of message A: a thinking block with a partial output count.
    let main = parse_chunk(&fs::read(&files[0]).unwrap());
    let a1 = &main.records[0];
    assert_eq!(a1.message_id, mid(1));
    assert_eq!(
        a1.request_id.as_deref(),
        Some("req_000000000000000000000001")
    );
    assert_eq!(a1.session_id, S1);
    assert_eq!(a1.agent_id, None);
    assert!(!a1.is_sidechain);
    assert_eq!(a1.ts, utc_ms(2026, 10, 27, 9, 0, 5) + 100);
    assert_eq!(a1.model, "claude-sonnet-5");
    assert_eq!(a1.project_path, "/Users/example/projects/demo");
    assert_eq!(
        (
            a1.input_tokens,
            a1.output_tokens,
            a1.cache_creation_tokens,
            a1.cache_read_tokens
        ),
        (6, 12, 1500, 40000)
    );
    // Output only grows across the lines of one message; the rest is constant.
    let outs: Vec<u64> = main.records[..3].iter().map(|r| r.output_tokens).collect();
    assert_eq!(outs, [12, 150, 380]);
    assert!(main.records[..3].iter().all(|r| r.message_id == mid(1)));

    // Subagent lines carry the parent session id, the flag and the agent id.
    let sub = parse_chunk(&fs::read(&files[2]).unwrap());
    assert!(sub
        .records
        .iter()
        .all(|r| r.is_sidechain && r.session_id == S1));
    assert!(sub
        .records
        .iter()
        .all(|r| r.agent_id.as_deref() == Some("a1b2c3d4e5f60718")));

    // The resumed file copies S1 lines with their original session id.
    let resumed = parse_chunk(&fs::read(&files[1]).unwrap());
    let sessions: Vec<&str> = resumed
        .records
        .iter()
        .map(|r| r.session_id.as_str())
        .collect();
    assert_eq!(sessions, [S1, S1, S1, S1, S2, S2, S2]);

    // The cwd can change inside a file (M4 runs in a sub directory).
    let s3 = parse_chunk(&fs::read(&files[3]).unwrap());
    assert_eq!(s3.records[0].project_path, "/Users/example/projects/other");
    assert_eq!(
        s3.records[4].project_path,
        "/Users/example/projects/other/sub"
    );
}

#[test]
fn totals_are_identical_for_every_file_order() {
    let files = main_set();
    let mut reference = None;
    for order in all_permutations(&files) {
        let (_dir, db) = ingest_in_order(&order);
        assert_eq!(totals(&db), TOTAL, "totals for order {order:?}");
        let rows = snapshot(&db);
        match &reference {
            None => reference = Some(rows),
            Some(first) => assert_eq!(&rows, first, "stored rows for order {order:?}"),
        }
    }
    assert_eq!(reference.unwrap().len(), 11);
}

#[test]
fn totals_are_identical_for_any_line_order_and_batching() {
    let mut records = Vec::new();
    for p in main_set() {
        records.extend(parse_chunk(&fs::read(p).unwrap()).records);
    }
    assert_eq!(records.len(), 20);

    let reference = {
        let mut db = Db::open_in_memory().unwrap();
        db.upsert_messages(&records).unwrap();
        (totals(&db), snapshot(&db))
    };
    assert_eq!(reference.0, TOTAL);

    // Reversed, in one batch.
    let mut reversed = records.clone();
    reversed.reverse();
    let mut db = Db::open_in_memory().unwrap();
    db.upsert_messages(&reversed).unwrap();
    assert_eq!((totals(&db), snapshot(&db)), reference);

    // Interleaved (even indexes first, then odd), one transaction per record.
    let mut db = Db::open_in_memory().unwrap();
    for r in records
        .iter()
        .step_by(2)
        .chain(records.iter().skip(1).step_by(2))
    {
        db.upsert_messages(std::slice::from_ref(r)).unwrap();
    }
    assert_eq!((totals(&db), snapshot(&db)), reference);

    // Every record offered twice is a no-op.
    let mut db = Db::open_in_memory().unwrap();
    db.upsert_messages(&records).unwrap();
    let again = db.upsert_messages(&records).unwrap();
    assert_eq!(again.inserted, 0);
    assert_eq!((totals(&db), snapshot(&db)), reference);
}

#[test]
fn dedup_keeps_max_output_and_min_timestamp() {
    // Read the cut-off copy (A with output 12 and 150) last, so a naive
    // "last write wins" would end up with 150.
    let files = main_set();
    let order = [
        files[0].clone(),
        files[2].clone(),
        files[3].clone(),
        files[1].clone(),
    ];
    let (_dir, db) = ingest_in_order(&order);
    let rows = snapshot(&db);
    let row = |k: u32| rows.iter().find(|r| r.0 == mid(k)).unwrap().clone();

    let a = row(1);
    assert_eq!(
        a.5,
        utc_ms(2026, 10, 27, 9, 0, 5) + 100,
        "A keeps the earliest line timestamp"
    );
    assert_eq!((a.8, a.9, a.10, a.11), (6, 380, 1500, 40000));

    // M3 has two lines 2.5 s apart; the first timestamp and the last count win.
    let m3 = row(10);
    assert_eq!(m3.5, utc_ms(2026, 10, 28, 8, 15, 0));
    assert_eq!((m3.8, m3.9, m3.10, m3.11), (4, 700, 1000, 250000));

    // The subagent message is flagged and attributed to the parent session.
    let s1x = row(6);
    assert!(s1x.4);
    assert_eq!(s1x.3.as_deref(), Some("a1b2c3d4e5f60718"));
    assert_eq!(s1x.2, S1);
    assert_eq!(s1x.9, 310);

    // Synthetic, error and id-less lines never reach the table.
    assert!(rows.iter().all(|r| r.0.starts_with("msg_")));
    assert!(rows.iter().all(|r| r.6 == "claude-sonnet-5"));
    assert!(rows.iter().all(|r| r.0 != mid(900) && r.0 != mid(901)));
}

#[test]
fn session_and_latest_queries_follow_dedup() {
    let (_dir, db) = ingest_in_order(&main_set());

    // S1 = A + B + C (main) + two subagent messages; the copies in the
    // resumed file keep the S1 id and add nothing.
    // input 6+8+4+10+3 = 31, output 380+220+90+310+75 = 1075,
    // cache creation 1500+300+0+5000+0 = 6800, cache read 40000+41500+42000+0+5000 = 128500.
    let s1 = db.session_summary(S1).unwrap().unwrap();
    assert_eq!(
        s1.totals,
        TokenTotals {
            messages: 5,
            input: 31,
            output: 1075,
            cache_creation: 6800,
            cache_read: 128_500,
        }
    );
    assert_eq!(s1.first_ts, utc_ms(2026, 10, 27, 9, 0, 5) + 100);
    assert_eq!(s1.last_ts, utc_ms(2026, 10, 27, 9, 5, 0));

    // S2 = D + E: input 5+3, output 500+60, cache creation 2000, cache read 45000+47000.
    let s2 = db.session_summary(S2).unwrap().unwrap();
    assert_eq!(
        s2.totals,
        TokenTotals {
            messages: 2,
            input: 8,
            output: 560,
            cache_creation: 2000,
            cache_read: 92_000,
        }
    );
    assert_eq!(s2.first_ts, utc_ms(2026, 10, 27, 10, 30, 0));
    assert_eq!(s2.last_ts, utc_ms(2026, 10, 27, 10, 31, 10));

    assert!(db
        .session_summary("00000000-0000-4000-8000-000000000000")
        .unwrap()
        .is_none());

    let latest = db.latest_message().unwrap().unwrap();
    assert_eq!(latest.session_id, S3);
    assert_eq!(latest.ts, utc_ms(2026, 10, 28, 11, 30, 0));
    assert_eq!(db.message_count().unwrap(), 11);
}

#[test]
fn headline_excludes_cache_reads() {
    // 51 input + 2785 output + 9800 cache creation = 12636; the 1140500
    // cache-read tokens are reported separately.
    assert_eq!(TOTAL.headline(), 12_636);
}

#[test]
fn rereading_unchanged_files_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = new_db(dir.path());
    for p in main_set() {
        let first = ingest_file(&mut db, &p, 5_000);
        assert_eq!(first.plan, ReadPlan::Restart);
        assert_eq!(first.offset, fs::metadata(&p).unwrap().len());
    }
    let before = snapshot(&db);
    for p in main_set() {
        let second = ingest_file(&mut db, &p, 5_000);
        assert_eq!(second.plan, ReadPlan::Unchanged);
    }
    assert_eq!(snapshot(&db), before);
    assert_eq!(db.list_source_files().unwrap().len(), 4);
}
