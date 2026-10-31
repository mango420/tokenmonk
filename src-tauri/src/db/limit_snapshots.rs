//! Stored limit readings (table `limit_snapshots`).

use rusqlite::{params, OptionalExtension};

use super::{Db, DbResult};
use crate::contracts::limits::{LimitReading, LimitWindow};
use crate::contracts::EpochMs;

/// A stored reading together with the id of the source that produced it.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredLimit {
    pub source: String,
    pub reading: LimitReading,
}

/// A stored 5-hour reading, reduced to its three columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiveHourSnapshot {
    pub at: EpochMs,
    /// Utilisation in percent (may exceed 100).
    pub pct: f64,
    pub resets_at: Option<EpochMs>,
}

fn window(pct: Option<f64>, resets_at: Option<EpochMs>) -> Option<LimitWindow> {
    pct.map(|utilization| LimitWindow {
        utilization,
        resets_at,
    })
}

impl Db {
    /// Stores a good reading if it is newer than the newest stored one
    /// (`fetchedAt` strictly greater; docs/ARCHITECTURE.md, "File source").
    /// An equal or older `fetchedAt` is a repeat poll of the same data (also
    /// when a window was dropped since) and adds no row. Returns whether a
    /// row was written. Check and write are one statement.
    pub fn insert_limit_snapshot(&self, source: &str, reading: &LimitReading) -> DbResult<bool> {
        let written = self.conn().execute(
            "INSERT INTO limit_snapshots \
             (fetched_at, source, five_hour_pct, five_hour_resets_at, weekly_pct, weekly_resets_at) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?6 \
             WHERE ?1 > COALESCE((SELECT MAX(fetched_at) FROM limit_snapshots), ?7)",
            params![
                reading.fetched_at,
                source,
                reading.five_hour.as_ref().map(|w| w.utilization),
                reading.five_hour.as_ref().and_then(|w| w.resets_at),
                reading.weekly.as_ref().map(|w| w.utilization),
                reading.weekly.as_ref().and_then(|w| w.resets_at),
                i64::MIN,
            ],
        )?;
        Ok(written > 0)
    }

    /// Stored 5-hour readings with `fetched_at >= from`, oldest first.
    /// Rows without a 5-hour value are skipped.
    pub fn five_hour_snapshots_since(&self, from: EpochMs) -> DbResult<Vec<FiveHourSnapshot>> {
        let mut stmt = self.conn().prepare_cached(
            "SELECT fetched_at, five_hour_pct, five_hour_resets_at FROM limit_snapshots \
             WHERE fetched_at >= ?1 AND five_hour_pct IS NOT NULL ORDER BY fetched_at, id",
        )?;
        let rows = stmt.query_map([from], |r| {
            Ok(FiveHourSnapshot {
                at: r.get(0)?,
                pct: r.get(1)?,
                resets_at: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The newest stored weekly reading with `fetched_at < before`, as
    /// `(utilisation, resetsAt)`.
    pub fn last_weekly_before(&self, before: EpochMs) -> DbResult<Option<(f64, Option<EpochMs>)>> {
        Ok(self
            .conn()
            .prepare_cached(
                "SELECT weekly_pct, weekly_resets_at FROM limit_snapshots \
                 WHERE fetched_at < ?1 AND weekly_pct IS NOT NULL \
                 ORDER BY fetched_at DESC, id DESC LIMIT 1",
            )?
            .query_row([before], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?)
    }

    /// The newest stored reading (restores the last good value after a restart).
    pub fn latest_limit_snapshot(&self) -> DbResult<Option<StoredLimit>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT source, fetched_at, five_hour_pct, five_hour_resets_at, \
                        weekly_pct, weekly_resets_at \
                 FROM limit_snapshots ORDER BY fetched_at DESC, id DESC LIMIT 1",
                [],
                |r| {
                    Ok(StoredLimit {
                        source: r.get(0)?,
                        reading: LimitReading {
                            fetched_at: r.get(1)?,
                            five_hour: window(r.get(2)?, r.get(3)?),
                            weekly: window(r.get(4)?, r.get(5)?),
                        },
                    })
                },
            )
            .optional()?)
    }

    /// Weekly utilisation at the start of a local day (`W0` in
    /// docs/ARCHITECTURE.md): the last snapshot before `day_start_ms`, else
    /// the first one at or after it. `window_start_ms` limits the search to
    /// the current weekly window when its start is known.
    pub fn weekly_baseline(
        &self,
        day_start_ms: EpochMs,
        window_start_ms: Option<EpochMs>,
    ) -> DbResult<Option<f64>> {
        let floor = window_start_ms.unwrap_or(i64::MIN);
        let conn = self.conn();
        let before: Option<f64> = conn
            .query_row(
                "SELECT weekly_pct FROM limit_snapshots \
                 WHERE fetched_at < ?1 AND fetched_at >= ?2 AND weekly_pct IS NOT NULL \
                 ORDER BY fetched_at DESC, id DESC LIMIT 1",
                [day_start_ms, floor],
                |r| r.get(0),
            )
            .optional()?;
        if before.is_some() {
            return Ok(before);
        }
        Ok(conn
            .query_row(
                "SELECT weekly_pct FROM limit_snapshots \
                 WHERE fetched_at >= ?1 AND fetched_at >= ?2 AND weekly_pct IS NOT NULL \
                 ORDER BY fetched_at ASC, id ASC LIMIT 1",
                [day_start_ms, floor],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Deletes readings older than `before_ms`; returns the number removed.
    pub fn prune_limit_snapshots(&self, before_ms: EpochMs) -> DbResult<usize> {
        Ok(self.conn().execute(
            "DELETE FROM limit_snapshots WHERE fetched_at < ?1",
            [before_ms],
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(at: i64, five: Option<f64>, weekly: Option<f64>) -> LimitReading {
        LimitReading {
            five_hour: five.map(|u| LimitWindow {
                utilization: u,
                resets_at: Some(at + 1000),
            }),
            weekly: weekly.map(|u| LimitWindow {
                utilization: u,
                resets_at: Some(at + 5000),
            }),
            fetched_at: at,
        }
    }

    #[test]
    fn latest_roundtrips_both_windows() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.latest_limit_snapshot().unwrap(), None);
        db.insert_limit_snapshot("s1", &reading(100, Some(10.0), None))
            .unwrap();
        let newest = reading(200, Some(20.5), Some(33.0));
        db.insert_limit_snapshot("s1", &newest).unwrap();
        let got = db.latest_limit_snapshot().unwrap().unwrap();
        assert_eq!(got.source, "s1");
        assert_eq!(got.reading, newest);
    }

    #[test]
    fn baseline_prefers_last_before_midnight_else_first_of_day() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.weekly_baseline(1000, None).unwrap(), None);
        db.insert_limit_snapshot("s", &reading(1100, None, Some(50.0)))
            .unwrap();
        db.insert_limit_snapshot("s", &reading(1200, None, Some(55.0)))
            .unwrap();
        // Nothing before the day start: first snapshot of the day.
        assert_eq!(db.weekly_baseline(1000, None).unwrap(), Some(50.0));
        // Older readings are not stored by the helper; seed them directly.
        for (at, pct) in [(900, 40.0), (950, 45.0)] {
            db.conn()
                .execute(
                    "INSERT INTO limit_snapshots (fetched_at, source, weekly_pct) \
                     VALUES (?1, 's', ?2)",
                    params![at, pct],
                )
                .unwrap();
        }
        assert_eq!(db.weekly_baseline(1000, None).unwrap(), Some(45.0));
        // A window that started at 960 excludes the older snapshots.
        assert_eq!(db.weekly_baseline(1000, Some(960)).unwrap(), Some(50.0));
    }

    fn count(db: &Db) -> i64 {
        db.conn()
            .query_row("SELECT COUNT(*) FROM limit_snapshots", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn only_a_newer_fetched_at_is_stored() {
        let db = Db::open_in_memory().unwrap();
        assert!(db
            .insert_limit_snapshot("s", &reading(100, Some(10.0), Some(5.0)))
            .unwrap());
        // Same reading again (idle poll): no row.
        assert!(!db
            .insert_limit_snapshot("s", &reading(100, Some(10.0), Some(5.0)))
            .unwrap());
        // Same fetchedAt with a window dropped: no row either.
        assert!(!db
            .insert_limit_snapshot("s", &reading(100, None, Some(5.0)))
            .unwrap());
        // Older fetchedAt: no row.
        assert!(!db
            .insert_limit_snapshot("s", &reading(99, Some(1.0), None))
            .unwrap());
        assert_eq!(count(&db), 1);
        // Newer: stored, and it becomes the newest.
        assert!(db
            .insert_limit_snapshot("s", &reading(101, None, Some(6.0)))
            .unwrap());
        assert_eq!(count(&db), 2);
        let newest = db.latest_limit_snapshot().unwrap().unwrap();
        assert_eq!(newest.reading, reading(101, None, Some(6.0)));
    }

    #[test]
    fn extreme_fetched_at_is_stored_in_an_empty_table() {
        let db = Db::open_in_memory().unwrap();
        assert!(db
            .insert_limit_snapshot("s", &reading(i64::MIN + 1, Some(1.0), None))
            .unwrap());
    }

    #[test]
    fn five_hour_snapshots_skip_rows_without_the_window() {
        let db = Db::open_in_memory().unwrap();
        for (at, five, weekly) in [
            (10, Some(10.0), None),
            (20, None, Some(5.0)),
            (30, Some(30.0), None),
        ] {
            db.insert_limit_snapshot("s", &reading(at, five, weekly))
                .unwrap();
        }
        let all = db.five_hour_snapshots_since(0).unwrap();
        assert_eq!(
            all,
            vec![
                FiveHourSnapshot {
                    at: 10,
                    pct: 10.0,
                    resets_at: Some(1010)
                },
                FiveHourSnapshot {
                    at: 30,
                    pct: 30.0,
                    resets_at: Some(1030)
                },
            ]
        );
        assert_eq!(db.five_hour_snapshots_since(11).unwrap().len(), 1);
    }

    #[test]
    fn last_weekly_before_picks_the_newest_earlier_weekly_row() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.last_weekly_before(1000).unwrap(), None);
        for (at, five, weekly) in [
            (10, None, Some(5.0)),
            (20, Some(1.0), None),
            (30, None, Some(7.0)),
        ] {
            db.insert_limit_snapshot("s", &reading(at, five, weekly))
                .unwrap();
        }
        assert_eq!(db.last_weekly_before(31).unwrap(), Some((7.0, Some(5030))));
        assert_eq!(db.last_weekly_before(30).unwrap(), Some((5.0, Some(5010))));
        assert_eq!(db.last_weekly_before(10).unwrap(), None);
    }

    #[test]
    fn prune_removes_old_rows() {
        let db = Db::open_in_memory().unwrap();
        for at in [10, 20, 30] {
            db.insert_limit_snapshot("s", &reading(at, Some(1.0), None))
                .unwrap();
        }
        assert_eq!(db.prune_limit_snapshots(25).unwrap(), 2);
        let newest = db.latest_limit_snapshot().unwrap().unwrap();
        assert_eq!(newest.reading.fetched_at, 30);
    }
}
