//! Owns the running ingest worker so the Claude folder can be switched live
//! (docs/ARCHITECTURE.md, "Claude folder").
//!
//! The limits worker keeps one sender for the whole run; a relay thread
//! forwards its readings to whichever ingest is current. A switch stops the
//! old ingest and waits for it (never two writers), then opens a fresh write
//! connection and starts the new worker on the new root.

use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use tauri::AppHandle;

use super::{AppPublisher, AppState, TauriSink};
use crate::db::Db;
use crate::limits::LimitPoll;
use crate::watcher::{spawn_ingest, ClockZone, IngestConfig, IngestHandle};

type Target = Arc<Mutex<Sender<LimitPoll>>>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Managed state: the current ingest worker and what is needed to replace it.
pub struct IngestSupervisor {
    app: AppHandle,
    state: Arc<AppState>,
    db_path: PathBuf,
    target: Target,
    current: Mutex<Option<IngestHandle>>,
}

impl IngestSupervisor {
    /// Starts the relay and the first ingest. `first_db` is the write
    /// connection opened at startup; `config` is `None` without a home folder.
    pub fn start(
        app: AppHandle,
        state: Arc<AppState>,
        db_path: PathBuf,
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
                    // A send error means the ingest is being replaced; the
                    // reading is dropped and the next poll reaches the new one.
                    let _ = lock(&relay_target).send(poll);
                }
            })?;
        let supervisor = Self {
            app,
            state,
            db_path,
            target,
            current: Mutex::new(None),
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
        let publisher = AppPublisher::new(
            TauriSink::new(self.app.clone()),
            Arc::clone(&self.state),
            ClockZone::Local,
        );
        spawn_ingest(config, db, rx, Box::new(publisher))
    }

    pub fn recompute(&self) {
        if let Some(h) = lock(&self.current).as_ref() {
            h.recompute();
        }
    }

    /// Replaces the ingest by one reading `config.logs_root`. Blocks until
    /// the old worker has ended, so call it off the UI thread.
    pub fn switch(&self, config: IngestConfig) -> Result<(), String> {
        let mut current = lock(&self.current);
        if let Some(old) = current.take() {
            old.stop_and_wait();
        }
        let db = Db::open(&self.db_path).map_err(|e| e.to_string())?;
        let (tx, rx) = channel();
        *lock(&self.target) = tx;
        let handle = self.spawn(config, db, rx).map_err(|e| e.to_string())?;
        *current = Some(handle);
        Ok(())
    }
}
