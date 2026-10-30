//! The seam between the ingest worker and the rest of the app.
//!
//! The app layer implements [`IngestPublisher`] to store the snapshot in
//! shared state, emit `dashboard-updated` and update the tray halo; the game
//! module is called from the same hooks (evaluate unlocks, award XP).
//! Both run on the ingest worker thread and may write through the `Db` they
//! are handed, which is the worker's single write connection. Keep the hooks
//! quick: a slow hook delays log ingestion.

use crate::contracts::dashboard::DashboardSnapshot;
use crate::db::Db;
use crate::logs::UsageRecord;

/// Why a snapshot was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotCause {
    /// Progress during the initial import (throttled); `data.state` is `importing`.
    Import,
    /// The initial scan finished. Sent exactly once, first start of the
    /// worker; `data.state` is `ready`, `noLogs` or `error`. The game's
    /// retroactive replay starts here.
    ImportDone,
    /// New log lines were ingested.
    Files,
    /// A limit poll arrived.
    Limits,
    /// The minute tick: day rollover, burn rate decay, session idle
    /// cut-off, season check.
    Tick,
    /// Asked for through [`super::IngestHandle`] (for example after a
    /// settings change).
    Requested,
}

/// Receives the worker's results. See the module docs for the contract.
pub trait IngestPublisher: Send {
    /// New records were committed. `live` is `false` only for the startup
    /// scan (the backlog present when the worker starts) and `true` for
    /// everything read afterwards, including logs that appear later (fresh
    /// install, folder recovering from an error). The slice holds what was parsed from
    /// one read, so it may repeat messages that were stored before.
    fn on_new_usage(&mut self, _db: &mut Db, _records: &[UsageRecord], _live: bool) {}

    /// A fresh snapshot is ready.
    fn on_snapshot(&mut self, db: &mut Db, snapshot: &DashboardSnapshot, cause: SnapshotCause);
}
