//! Owns the running ingest worker so the Claude folder can be switched live
//! (docs/ARCHITECTURE.md, "Claude folder").
//!
//! The limits worker keeps one sender for the whole run; a relay thread
//! forwards its readings to whichever ingest is current. A switch first
//! points the relay at the new worker's inbox (readings wait there), then
//! stops the old worker and waits for it (never two writers), opens a fresh
//! write connection and starts the new worker on the new root.

use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use crate::db::Db;
use crate::limits::LimitPoll;
use crate::watcher::{spawn_ingest, IngestConfig, IngestHandle, IngestPublisher};

type Target = Arc<Mutex<Sender<LimitPoll>>>;
/// Builds the publisher of a new worker.
pub type PublisherFactory = Box<dyn Fn() -> Box<dyn IngestPublisher + Send> + Send + Sync>;
/// Opens a fresh write connection for a new worker.
pub type DbOpener = Box<dyn Fn() -> Result<Db, String> + Send + Sync>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Managed state: the current ingest worker and what is needed to replace it.
pub struct IngestSupervisor {
    publisher: PublisherFactory,
    open_db: DbOpener,
    target: Target,
    current: Mutex<Option<IngestHandle>>,
    /// Serializes switches. Never held together with a long wait on
    /// `current`, so `recompute` stays instant.
    switching: Mutex<()>,
}

impl IngestSupervisor {
    /// Starts the relay and the first ingest. `first_db` is the write
    /// connection opened at startup; `config` is `None` without a home folder.
    pub fn start(
        publisher: PublisherFactory,
        open_db: DbOpener,
        limit_rx: Receiver<LimitPoll>,
        first_db: Db,
        config: Option<IngestConfig>,
    ) -> io::Result<Self> {
        let (tx, rx) = channel();
        let target: Target = Arc::new(Mutex::new(tx));
        let relay_target = Arc::clone(&target);
        thread::Builder::new()
            .name("limits-relay".into())
            .spawn(move || {
                for poll in limit_rx {
                    // A send error means ingest is off after a failed
                    // switch; the reading is dropped.
                    let _ = lock(&relay_target).send(poll);
                }
            })?;
        let supervisor = Self {
            publisher,
            open_db,
            target,
            current: Mutex::new(None),
            switching: Mutex::new(()),
        };
        match config {
            Some(config) => {
                let handle = supervisor.spawn(config, first_db, rx)?;
                *lock(&supervisor.current) = Some(handle);
            }
            None => log::warn!("home directory unknown: log ingestion is off"),
        }
        Ok(supervisor)
    }

    fn spawn(
        &self,
        config: IngestConfig,
        db: Db,
        rx: Receiver<LimitPoll>,
    ) -> io::Result<IngestHandle> {
        spawn_ingest(config, db, rx, (self.publisher)())
    }

    /// A recompute while no worker runs is a no-op.
    pub fn recompute(&self) {
        if let Some(h) = lock(&self.current).as_ref() {
            h.recompute();
        }
    }

    /// Replaces the ingest by one on the config `resolve` returns. `resolve`
    /// runs after the switch lock is taken, so it sees the newest settings
    /// and the last change wins; `None` leaves the ingest as it is. Blocks
    /// until the old worker has ended, so call it off the main thread.
    ///
    /// On `Err` the old worker is gone and ingestion is off.
    pub fn switch(&self, resolve: impl FnOnce() -> Option<IngestConfig>) -> Result<(), String> {
        let _switching = lock(&self.switching);
        let Some(config) = resolve() else {
            return Ok(());
        };
        // Readings arriving from now on wait for the new worker.
        let (tx, rx) = channel();
        *lock(&self.target) = tx;
        let old = lock(&self.current).take();
        if let Some(old) = old {
            old.stop_and_wait();
        }
        let db = (self.open_db)()?;
        let handle = self.spawn(config, db, rx).map_err(|e| e.to_string())?;
        *lock(&self.current) = Some(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{DashboardSnapshot, LimitStatus};
    use crate::contracts::{LimitReading, LimitWindow};
    use crate::limits::LimitPoll;
    use crate::watcher::{ClockZone, SnapshotCause};
    use chrono::FixedOffset;
    use std::time::Duration;

    #[derive(Debug)]
    enum Ev {
        Snapshot(Box<DashboardSnapshot>, SnapshotCause),
        Dropped(usize),
        Opened,
    }

    struct Pub {
        id: usize,
        tx: Sender<Ev>,
    }

    impl IngestPublisher for Pub {
        fn on_snapshot(&mut self, _: &mut Db, s: &DashboardSnapshot, c: SnapshotCause) {
            let _ = self.tx.send(Ev::Snapshot(Box::new(s.clone()), c));
        }
    }

    impl Drop for Pub {
        fn drop(&mut self) {
            let _ = self.tx.send(Ev::Dropped(self.id));
        }
    }

    fn config(root: &std::path::Path) -> IngestConfig {
        IngestConfig {
            logs_root: root.to_path_buf(),
            rescan_interval: Duration::from_millis(300),
            tick_interval: Duration::from_millis(250),
            debounce: Duration::from_millis(40),
            zone: ClockZone::Fixed(FixedOffset::east_opt(0).unwrap()),
        }
    }

    fn next(rx: &Receiver<Ev>) -> Ev {
        rx.recv_timeout(Duration::from_secs(10)).expect("event")
    }

    #[test]
    fn switch_stops_the_old_worker_first_and_keeps_limit_readings() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("t.db");
        let first = Db::open(&db_path).unwrap();
        let (ev_tx, ev_rx) = channel();
        let (limit_tx, limit_rx) = channel::<LimitPoll>();

        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let factory: PublisherFactory = {
            let tx = ev_tx.clone();
            Box::new(move || {
                let id = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Box::new(Pub { id, tx: tx.clone() })
            })
        };
        let opener: DbOpener = {
            let tx = ev_tx.clone();
            let path = db_path.clone();
            let limit_tx = limit_tx.clone();
            Box::new(move || {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                let _ = tx.send(Ev::Opened);
                // A reading that arrives while the switch is running.
                limit_tx
                    .send(LimitPoll {
                        source: "test",
                        result: Ok(LimitReading {
                            five_hour: Some(LimitWindow {
                                utilization: 41.0,
                                resets_at: Some(now + 3_600_000),
                            }),
                            weekly: None,
                            fetched_at: now,
                        }),
                    })
                    .unwrap();
                Db::open(&path).map_err(|e| e.to_string())
            })
        };

        let sup = IngestSupervisor::start(
            factory,
            opener,
            limit_rx,
            first,
            Some(config(&dir.path().join("a"))),
        )
        .unwrap();
        // Wait for the first worker's startup snapshot.
        loop {
            if let Ev::Snapshot(_, SnapshotCause::ImportDone) = next(&ev_rx) {
                break;
            }
        }

        sup.switch(|| Some(config(&dir.path().join("b")))).unwrap();

        // Old publisher is dropped (worker ended) before the new DB opens.
        let mut order = Vec::new();
        let mut limit_seen = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !limit_seen {
            assert!(
                std::time::Instant::now() < deadline,
                "limit reading never reached the new worker: {order:?}"
            );
            match next(&ev_rx) {
                Ev::Dropped(id) => order.push(format!("dropped{id}")),
                Ev::Opened => order.push("opened".into()),
                Ev::Snapshot(s, _) => {
                    limit_seen = s.limits.status == LimitStatus::Ok
                        && s.limits.five_hour.is_some_and(|w| w.utilization == 41.0);
                }
            }
        }
        let dropped = order.iter().position(|e| e == "dropped0").unwrap();
        let opened = order.iter().position(|e| e == "opened").unwrap();
        assert!(dropped < opened, "{order:?}");
        drop(sup);
    }

    #[test]
    fn a_failed_switch_leaves_ingest_off_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let first = Db::open_in_memory().unwrap();
        let (_limit_tx, limit_rx) = channel::<LimitPoll>();
        let (ev_tx, _ev_rx) = channel();
        let factory: PublisherFactory = Box::new(move || {
            Box::new(Pub {
                id: 0,
                tx: ev_tx.clone(),
            })
        });
        let opener: DbOpener = Box::new(|| Err("no database".into()));
        let sup = IngestSupervisor::start(
            factory,
            opener,
            limit_rx,
            first,
            Some(config(&dir.path().join("a"))),
        )
        .unwrap();
        assert!(sup.switch(|| Some(config(dir.path()))).is_err());
        sup.recompute();
    }
}
