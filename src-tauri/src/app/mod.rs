//! The app layer: shared state, the ingest publisher and the startup wiring
//! (docs/ARCHITECTURE.md, "Threads" and "Data flow").
//!
//! * [`AppState`]: cached snapshot, game state, settings and app info.
//! * [`AppPublisher`]: runs on the ingest worker, fills the cache, emits
//!   events, drives the tray halo and the game.
//! * [`setup`]: opens the database, builds the state, creates the tray and
//!   starts the limits and ingest workers.

mod facts;
mod game_store;
mod publisher;
mod state;

use std::sync::Arc;

use tauri::{App, AppHandle, Emitter, Manager};

pub use game_store::DbGameStore;
pub use publisher::{halo_percent, AppPublisher, Sink};
pub use state::AppState;

use crate::contracts::events::{
    ACHIEVEMENT_UNLOCKED, APP_INFO_CHANGED, DASHBOARD_UPDATED, GAME_UPDATED, LEVEL_UP,
};
use crate::contracts::{
    AchievementUnlocked, AppInfo, DashboardSnapshot, GameState, LevelUp, Mode, Platform, Season,
};
use crate::db::Db;
use crate::limits;
use crate::tray;
use crate::watcher::{spawn_ingest, ClockZone, IngestConfig, IngestHandle};
use crate::window;

/// File name of the database inside the app data directory.
pub const DB_FILE: &str = "tokenmonk.db";

/// Sends the publisher's results to the windows and the tray.
pub struct TauriSink {
    app: AppHandle,
}

impl TauriSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn emit<T: serde::Serialize + Clone>(&self, event: &str, payload: &T) {
        if let Err(e) = self.app.emit(event, payload) {
            log::warn!("could not emit {event}: {e}");
        }
    }
}

impl Sink for TauriSink {
    fn dashboard(&self, snapshot: &DashboardSnapshot) {
        self.emit(DASHBOARD_UPDATED, snapshot);
    }
    fn game(&self, state: &GameState) {
        self.emit(GAME_UPDATED, state);
    }
    fn achievement(&self, unlocked: &AchievementUnlocked) {
        self.emit(ACHIEVEMENT_UNLOCKED, unlocked);
    }
    fn level_up(&self, level_up: &LevelUp) {
        self.emit(LEVEL_UP, level_up);
    }
    fn app_info(&self, info: &AppInfo) {
        self.emit(APP_INFO_CHANGED, info);
    }
    fn halo(&self, percent: Option<f64>) {
        tray::set_halo(&self.app, percent);
    }
}

fn platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::Macos
    } else if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// Startup wiring, called from `lib.rs` once the plugins are registered.
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    let dir = handle.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join(DB_FILE);
    let (write_db, moved_aside) = Db::open_or_rebuild(&db_path)?;
    if moved_aside.is_some() {
        log::warn!("the database was unusable and has been rebuilt from the logs");
    }
    let command_db = Db::open(&db_path)?;
    let settings = write_db.settings()?;

    let config = IngestConfig::standard();
    let info = AppInfo {
        version: handle.package_info().version.to_string(),
        platform: platform(),
        season: Season::Regular,
        schema_version: write_db.schema_version()?,
        logs_found: config.as_ref().is_some_and(|c| c.logs_root.is_dir()),
    };
    let first_run = settings.mode == Mode::NoneYet;
    let state = Arc::new(AppState::new(command_db, settings, info));
    app.manage(Arc::clone(&state));

    window::init(&handle)?;
    tray::init(&handle)?;

    let (limit_tx, limit_rx) = std::sync::mpsc::channel();
    let limits_handle = limits::spawn(limits::default_source(), limit_tx)?;
    app.manage(limits_handle);

    match config {
        Some(config) => {
            let publisher = AppPublisher::new(
                TauriSink::new(handle.clone()),
                Arc::clone(&state),
                ClockZone::Local,
            );
            let ingest: IngestHandle =
                spawn_ingest(config, write_db, limit_rx, Box::new(publisher))?;
            app.manage(ingest);
        }
        None => log::warn!("home directory unknown: log ingestion is off"),
    }

    // Onboarding waits in the popup; a tray icon alone would hide it.
    if first_run {
        window::show(&handle);
    }
    Ok(())
}
