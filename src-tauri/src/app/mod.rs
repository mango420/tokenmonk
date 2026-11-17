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
mod ingest;
mod publisher;
mod state;
mod updates;

use std::sync::Arc;

use tauri::{App, AppHandle, Emitter, Manager};

pub use game_store::DbGameStore;
pub use ingest::IngestSupervisor;
pub use publisher::{halo_percent, AppPublisher, Sink};
pub use state::AppState;
pub use updates::{check as check_update, install as install_update, UpdateState};

use crate::contracts::events::{
    ACHIEVEMENT_UNLOCKED, APP_INFO_CHANGED, DASHBOARD_UPDATED, GAME_UPDATED, LEVEL_UP,
};
use crate::contracts::{
    AchievementUnlocked, AppInfo, DashboardSnapshot, GameState, LevelUp, Mode, Platform,
};
use crate::db::Db;
use crate::limits;
use crate::tray;
use crate::watcher::{claude_folder, ClockZone, IngestConfig};
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
    fn halo(&self, percent: Option<f64>, mode: Mode) {
        tray::set_halo(&self.app, percent, mode);
    }
    fn popup_visible(&self) -> bool {
        window::is_visible(&self.app)
    }
    fn notify(&self, title: &str, body: &str) {
        use tauri_plugin_notification::NotificationExt;
        if let Err(e) = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
        {
            log::warn!("could not show a notification: {e}");
        }
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

/// `app_state` key set once the first-start popup has been decided.
const KEY_FIRST_START_DONE: &str = "first_start_done";

/// True while the first-start popup is still owed: the flag is unset and the
/// database is new. A database that existed before this start belongs to an
/// install from before the flag, so it is marked done silently instead.
fn first_start_pending(db: &Db, db_existed: bool) -> bool {
    match db.app_state(KEY_FIRST_START_DONE) {
        Ok(Some(_)) => false,
        Ok(None) if db_existed => {
            mark_first_start_done(db);
            false
        }
        Ok(None) => true,
        Err(e) => {
            log::warn!("could not read the first-start flag: {e}");
            false
        }
    }
}

fn mark_first_start_done(db: &Db) {
    if let Err(e) = db.set_app_state(KEY_FIRST_START_DONE, &serde_json::Value::Bool(true)) {
        log::warn!("could not store the first-start flag: {e}");
    }
}

/// Startup wiring, called from `lib.rs` once the plugins are registered.
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    let dir = handle.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join(DB_FILE);
    let db_existed = db_path.exists();
    let (write_db, moved_aside) = Db::open_or_rebuild(&db_path)?;
    if moved_aside.is_some() {
        log::warn!("the database was unusable and has been rebuilt from the logs");
    }
    let command_db = Db::open(&db_path)?;
    let settings = write_db.settings()?;

    let folder = claude_folder(settings.claude_folder.as_deref());
    let config = IngestConfig::for_setting(settings.claude_folder.as_deref());
    let info = AppInfo {
        version: handle.package_info().version.to_string(),
        platform: platform(),
        schema_version: write_db.schema_version()?,
        logs_found: folder.projects_found,
    };
    let first_run = first_start_pending(&command_db, db_existed);
    let update_check = settings.update_check;
    let state = Arc::new(AppState::new(command_db, settings, info));
    app.manage(Arc::clone(&state));

    window::init(&handle)?;
    tray::init(&handle)?;

    let (limit_tx, limit_rx) = std::sync::mpsc::channel();
    let limits_handle = limits::spawn(limits::default_source(), limit_tx)?;
    app.manage(limits_handle);

    let publisher_app = handle.clone();
    let publisher_state = Arc::clone(&state);
    let ingest = IngestSupervisor::start(
        Box::new(move || {
            Box::new(AppPublisher::new(
                TauriSink::new(publisher_app.clone()),
                Arc::clone(&publisher_state),
                ClockZone::Local,
            ))
        }),
        Box::new(move || Db::open(&db_path).map_err(|e| e.to_string())),
        limit_rx,
        write_db,
        config,
    )?;
    app.manage(ingest);

    updates::init(&handle, update_check);

    // A brand-new install opens the popup once; a tray icon alone would hide it.
    if first_run {
        window::show(&handle);
        // Only now: an earlier setup failure must not use up the popup.
        if let Err(e) = state.with_db(|db| {
            mark_first_start_done(db);
            Ok(())
        }) {
            log::warn!("could not store the first-start flag: {e}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_is_owed_until_marked_on_a_new_install() {
        let db = Db::open_in_memory().unwrap();
        assert!(first_start_pending(&db, false));
        assert!(first_start_pending(&db, false));
        mark_first_start_done(&db);
        assert!(!first_start_pending(&db, false));
    }

    #[test]
    fn existing_install_gets_no_popup() {
        let db = Db::open_in_memory().unwrap();
        assert!(!first_start_pending(&db, true));
        assert!(db.app_state(KEY_FIRST_START_DONE).unwrap().is_some());
        assert!(!first_start_pending(&db, false));
    }
}
