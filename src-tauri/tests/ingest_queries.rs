//! Day and window queries over the ingested fixtures. Expected numbers are
//! computed by hand from the message table in `ingest_fixtures.rs`.
//!
//! Message times (UTC) and tokens (input / output / cache creation / cache read):
//!   2026-10-27  09:00:05.100 A    6 /  380 / 1500 /  40000
//!               09:00:20     B    8 /  220 /  300 /  41500
//!               09:03:00     S1x 10 /  310 / 5000 /      0   (subagent)
//!               09:03:30     S2x  3 /   75 /    0 /   5000   (subagent)
//!               09:05:00     C    4 /   90 /    0 /  42000
//!               10:30:00     D    5 /  500 / 2000 /  45000
//!               10:31:10     E    3 /   60 /    0 /  47000
//!               21:59:59.999 M1   2 /  100 /    0 / 200000
//!               22:00:00.000 M2   2 /   50 /    0 / 210000
//!   2026-10-28  08:15:00     M3   4 /  700 / 1000 / 250000
//!               11:30:00     M4   4 /  300 /    0 / 260000

mod common;

use std::path::PathBuf;

use chrono::{FixedOffset, NaiveDate, Utc};
use common::*;
use tokenmonk_lib::db::{Db, TokenTotals};

fn loaded() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let mut db = new_db(dir.path());
    let mut files: Vec<PathBuf> = main_set();
    files.reverse(); // order must not matter
    for (i, p) in files.iter().enumerate() {
        ingest_file(&mut db, p, 100 + i as i64);
    }
    (dir, db)
}

fn t(messages: u64, input: u64, output: u64, cache_creation: u64, cache_read: u64) -> TokenTotals {
    TokenTotals {
        messages,
        input,
        output,
        cache_creation,
        cache_read,
    }
}

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn range() -> (i64, i64) {
    (utc_ms(2026, 10, 20, 0, 0, 0), utc_ms(2026, 11, 3, 0, 0, 0))
}

#[test]
fn local_days_east_of_utc_split_at_local_midnight() {
    let (_dir, db) = loaded();
    let tz = FixedOffset::east_opt(2 * 3600).unwrap(); // local midnight = 22:00 UTC
    let (from, to) = range();
    let days = db.tokens_by_local_day(&tz, from, to).unwrap();
    assert_eq!(days.len(), 2);

    // 2026-10-27 local = 10-26T22:00Z .. 10-27T22:00Z: A, B, S1x, S2x, C, D, E, M1
    // (M1 at 21:59:59.999Z is 23:59:59.999 local).
    // input 6+8+10+3+4+5+3+2 = 41, output 380+220+310+75+90+500+60+100 = 1735,
    // cache creation 1500+300+5000+0+0+2000+0+0 = 8800,
    // cache read 40000+41500+0+5000+42000+45000+47000+200000 = 420500.
    assert_eq!(days[0].day, day(2026, 10, 27));
    assert_eq!(days[0].totals, t(8, 41, 1735, 8800, 420_500));

    // 2026-10-28 local: M2 (exactly local midnight), M3, M4.
    // input 2+4+4 = 10, output 50+700+300 = 1050, cache creation 1000,
    // cache read 210000+250000+260000 = 720000.
    assert_eq!(days[1].day, day(2026, 10, 28));
    assert_eq!(days[1].totals, t(3, 10, 1050, 1000, 720_000));
}

#[test]
fn local_days_in_utc_move_the_midnight_message() {
    let (_dir, db) = loaded();
    let (from, to) = range();
    let days = db.tokens_by_local_day(&Utc, from, to).unwrap();
    assert_eq!(days.len(), 2);
    // 10-27 UTC: the +02:00 day plus M2 (22:00Z is still the 27th).
    // input 41+2 = 43, output 1735+50 = 1785, cache creation 8800,
    // cache read 420500+210000 = 630500.
    assert_eq!(days[0].day, day(2026, 10, 27));
    assert_eq!(days[0].totals, t(9, 43, 1785, 8800, 630_500));
    // 10-28 UTC: M3 and M4: input 8, output 1000, cache creation 1000,
    // cache read 250000+260000 = 510000.
    assert_eq!(days[1].day, day(2026, 10, 28));
    assert_eq!(days[1].totals, t(2, 8, 1000, 1000, 510_000));
}

#[test]
fn local_days_far_east_give_three_days() {
    let (_dir, db) = loaded();
    let tz = FixedOffset::east_opt(14 * 3600).unwrap(); // local midnight = 10:00 UTC
    let (from, to) = range();
    let days = db.tokens_by_local_day(&tz, from, to).unwrap();
    let got: Vec<_> = days.iter().map(|d| (d.day, d.totals)).collect();
    assert_eq!(
        got,
        [
            // 10-27 local = 10-26T10:00Z .. 10-27T10:00Z: A, B, S1x, S2x, C
            // input 6+8+10+3+4 = 31, output 380+220+310+75+90 = 1075,
            // cache creation 1500+300+5000 = 6800, cache read 40000+41500+0+5000+42000 = 128500.
            (day(2026, 10, 27), t(5, 31, 1075, 6800, 128_500)),
            // 10-28 local = 10-27T10:00Z .. 10-28T10:00Z: D, E, M1, M2, M3
            // input 5+3+2+2+4 = 16, output 500+60+100+50+700 = 1410,
            // cache creation 2000+1000 = 3000, cache read 45000+47000+200000+210000+250000 = 752000.
            (day(2026, 10, 28), t(5, 16, 1410, 3000, 752_000)),
            // 10-29 local = from 10-28T10:00Z: M4 only.
            (day(2026, 10, 29), t(1, 4, 300, 0, 260_000)),
        ]
    );
}

#[test]
fn day_range_is_half_open_at_local_midnight() {
    let (_dir, db) = loaded();
    let tz = FixedOffset::east_opt(2 * 3600).unwrap();
    let midnight = utc_ms(2026, 10, 27, 22, 0, 0); // 2026-10-28 00:00 local
    let next_midnight = utc_ms(2026, 10, 28, 22, 0, 0);

    // [midnight, next midnight) holds exactly the 28th: M2, M3, M4.
    let day28 = t(3, 10, 1050, 1000, 720_000);
    assert_eq!(db.tokens_between(midnight, next_midnight).unwrap(), day28);
    let by_day = db
        .tokens_by_local_day(&tz, midnight, next_midnight)
        .unwrap();
    assert_eq!(by_day.len(), 1);
    assert_eq!(
        (by_day[0].day, by_day[0].totals),
        (day(2026, 10, 28), day28)
    );

    // One millisecond earlier pulls M1 (23:59:59.999 on the 27th) in.
    let with_m1 = db.tokens_between(midnight - 1, next_midnight).unwrap();
    assert_eq!(with_m1, t(4, 12, 1150, 1000, 920_000));
    // The upper bound is exclusive: ending exactly at M2 leaves it out.
    assert_eq!(
        db.tokens_between(midnight - 1, midnight).unwrap().messages,
        1
    );
}

#[test]
fn trailing_windows_match_hand_computed_totals() {
    let (_dir, db) = loaded();
    let now = utc_ms(2026, 10, 28, 12, 0, 0);
    let hour = 3_600_000_i64;

    // 24 h: from 10-27T12:00Z: M1, M2, M3, M4 (D and E at 10:30Z are out).
    // input 2+2+4+4 = 12, output 100+50+700+300 = 1150, cache creation 1000,
    // cache read 200000+210000+250000+260000 = 920000.
    let day = db.tokens_trailing(now, 24 * hour).unwrap();
    assert_eq!(day, t(4, 12, 1150, 1000, 920_000));
    // Cache reads dwarf the headline number (12 + 1150 + 1000).
    assert_eq!(day.headline(), 2162);
    assert!(day.cache_read > 400 * day.headline());

    // 5 h: from 07:00Z: M3 and M4. input 8, output 1000, cache creation 1000,
    // cache read 510000.
    assert_eq!(
        db.tokens_trailing(now, 5 * hour).unwrap(),
        t(2, 8, 1000, 1000, 510_000)
    );

    // Nothing in the last 15 minutes before noon (M4 is at 11:30).
    assert_eq!(
        db.tokens_trailing(now, hour / 4).unwrap(),
        TokenTotals::default()
    );
}

#[test]
fn trailing_window_bounds_are_inclusive_and_ignore_the_future() {
    let (_dir, db) = loaded();
    let m4 = utc_ms(2026, 10, 28, 11, 30, 0);
    let m3 = utc_ms(2026, 10, 28, 8, 15, 0);
    let span = m4 - m3; // 3 h 15 min

    // The window start may equal a message time: M3 and M4.
    assert_eq!(db.tokens_trailing(m4, span).unwrap().messages, 2);
    // One millisecond shorter leaves M3 out: M4 only.
    assert_eq!(
        db.tokens_trailing(m4, span - 1).unwrap(),
        t(1, 4, 300, 0, 260_000)
    );

    // "now" inside session 1: C (09:05:00) lies in the future and is not counted,
    // the subagent messages are. now = 09:04:00Z, window 10 min (from 08:54Z).
    // A, B, S1x, S2x: input 6+8+10+3 = 27, output 380+220+310+75 = 985,
    // cache creation 1500+300+5000+0 = 6800, cache read 40000+41500+0+5000 = 86500.
    let now = utc_ms(2026, 10, 27, 9, 4, 0);
    assert_eq!(
        db.tokens_trailing(now, 10 * 60_000).unwrap(),
        t(4, 27, 985, 6800, 86_500)
    );
}
