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

fn window(pct: Option<f64>, resets_at: Option<EpochMs>) -> Option<LimitWindow> {
    pct.map(|utilization| LimitWindow {
        utilization,
        resets_at,
    })
}

impl Db {
    /// Appends a good reading.
    pub fn insert_limit_snapshot(&self, source: &str, reading: &LimitReading) -> DbResult<()> {
        self.conn().execute(
            "INSERT INTO limit_snapshots \
             (fetched_at, source, five_hour_pct, five_hour_resets_at, weekly_pct, weekly_resets_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                reading.fetched_at,
                source,
                reading.five_hour.as_ref().map(|w| w.utilization),
                reading.five_hour.as_ref().and_then(|w| w.resets_at),
                reading.weekly.as_ref().map(|w| w.utilization),
                reading.weekly.as_ref().and_then(|w| w.resets_at),
            ],
        )?;
        Ok(())
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
        db.insert_limit_snapshot("s", &reading(900, None, Some(40.0)))
            .unwrap();
        db.insert_limit_snapshot("s", &reading(950, None, Some(45.0)))
            .unwrap();
        assert_eq!(db.weekly_baseline(1000, None).unwrap(), Some(45.0));
        // A window that started at 960 excludes the older snapshots.
        assert_eq!(db.weekly_baseline(1000, Some(960)).unwrap(), Some(50.0));
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
