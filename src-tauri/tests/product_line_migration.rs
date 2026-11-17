//! TKM-301: an install that stored mode `diy` is switched to `max` silently
//! on the next start; XP, levels, achievements and history stay.
//!
//! The old install is built by hand: a database at the previous schema
//! version with raw rows exactly as v0.6 wrote them (fake data only).

mod common;

use std::path::Path;

use common::*;
use rusqlite::params;
use tokenmonk_lib::contracts::Mode;
use tokenmonk_lib::db::Db;
use tokenmonk_lib::game::levels::level_info;

/// Schema version v0.6 shipped (`schema_v1.sql`).
const V06_SCHEMA_VERSION: u32 = 1;

const XP_ROWS: &[(&str, &str, i64)] = &[
    ("day", "2026-10-25", 120),
    ("day", "2026-10-26", 340),
    ("block", "2026-10-27T08", 900),
    ("day", "2026-10-28", 2_500),
];

/// Builds a v0.6 database file: settings with the given mode, an XP ledger
/// whose rows carry that mode, one unlocked achievement, one app_state flag.
fn old_install(path: &Path, mode: &str) {
    let db = Db::open(path).expect("create");
    let c = db.conn();
    c.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('mode', ?1)",
        params![format!("\"{mode}\"")],
    )
    .unwrap();
    c.execute(
        "INSERT INTO settings (key, value) VALUES ('sound', 'false')",
        [],
    )
    .unwrap();
    for (kind, reference, amount) in XP_ROWS {
        c.execute(
            "INSERT INTO xp_events (earned_at, mode, kind, ref, amount, retroactive) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![utc_ms(2026, 10, 28, 9, 0, 0), mode, kind, reference, amount],
        )
        .unwrap();
    }
    c.execute(
        "INSERT INTO achievements (id, unlocked_at, retroactive, updated_at) \
         VALUES ('firstBlood', ?1, 0, ?1)",
        params![utc_ms(2026, 10, 25, 9, 0, 0)],
    )
    .unwrap();
    c.execute(
        "INSERT INTO app_state (key, value) VALUES ('retro_done', 'true')",
        [],
    )
    .unwrap();
    // Pretend nothing newer than v0.6 has run on this file.
    c.execute_batch(&format!("PRAGMA user_version = {V06_SCHEMA_VERSION}"))
        .unwrap();
}

fn xp_total_raw(db: &Db) -> i64 {
    db.conn()
        .query_row("SELECT COALESCE(SUM(amount),0) FROM xp_events", [], |r| {
            r.get(0)
        })
        .unwrap()
}

fn xp_sum() -> i64 {
    XP_ROWS.iter().map(|r| r.2).sum()
}

#[test]
fn diy_install_becomes_max_and_keeps_xp_levels_and_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tokenmonk-test.db");
    old_install(&path, "diy");
    let total = xp_sum();
    let level_before = level_info(total as u64).level;

    let db = Db::open(&path).expect("reopen runs the migration");

    assert_eq!(db.settings().unwrap().mode, Mode::Max);
    assert_eq!(
        db.setting("mode").unwrap().unwrap(),
        serde_json::json!("max")
    );
    // Unrelated settings are untouched.
    assert!(!db.settings().unwrap().sound);

    // XP: same total, same rows, same level.
    assert_eq!(xp_total_raw(&db), total);
    assert_eq!(db.xp_total().unwrap(), total);
    assert_eq!(db.xp_events().unwrap().len(), XP_ROWS.len());
    assert_eq!(
        level_info(db.xp_total().unwrap() as u64).level,
        level_before
    );

    // The ledger keeps its history: the rows still say what they said.
    let diy_rows: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM xp_events WHERE mode = 'diy'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(diy_rows, XP_ROWS.len() as i64);

    // Achievements and internal flags survive.
    let a = db.achievement("firstBlood").unwrap().expect("kept");
    assert!(a.unlocked_at.is_some());
    assert!(db.app_state("retro_done").unwrap().is_some());
}

#[test]
fn migration_runs_once_and_a_second_start_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tokenmonk-test.db");
    old_install(&path, "diy");

    drop(Db::open(&path).unwrap());
    let db = Db::open(&path).unwrap();

    assert_eq!(db.settings().unwrap().mode, Mode::Max);
    assert_eq!(xp_total_raw(&db), xp_sum());
    // Not sent back through onboarding.
    assert_ne!(db.settings().unwrap().mode, Mode::NoneYet);
}

#[test]
fn max_install_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tokenmonk-test.db");
    old_install(&path, "max");

    let db = Db::open(&path).unwrap();

    assert_eq!(db.settings().unwrap().mode, Mode::Max);
    assert_eq!(xp_total_raw(&db), xp_sum());
}

#[test]
fn fresh_install_starts_in_max() {
    let dir = tempfile::tempdir().unwrap();
    let db = new_db(dir.path());
    assert_eq!(db.settings().unwrap().mode, Mode::Max);
}
