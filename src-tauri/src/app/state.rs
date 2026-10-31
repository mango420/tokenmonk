//! Shared application state (docs/ARCHITECTURE.md, "Threads").
//!
//! The ingest worker publishes the newest snapshot and game state here;
//! command handlers only read the cache. Commands that write (settings, the
//! easter egg) use their own connection, guarded by a mutex; SQLite's WAL
//! mode and busy timeout keep that apart from the ingest worker's writes.

use std::sync::{Mutex, MutexGuard, RwLock};

use crate::contracts::{AppInfo, DashboardSnapshot, GameState, Settings};
use crate::db::{Db, DbResult};

#[derive(Debug)]
struct Cache {
    dashboard: Option<DashboardSnapshot>,
    game: Option<GameState>,
    settings: Settings,
    info: AppInfo,
}

/// Cached snapshot, game state, settings and app info, plus the command
/// connection. Put it in Tauri managed state inside an `Arc`.
pub struct AppState {
    cache: RwLock<Cache>,
    command_db: Mutex<Db>,
    /// Held while the game is evaluated and persisted, so the ingest worker
    /// and the easter egg command never interleave their load/persist.
    pub game_lock: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl AppState {
    pub fn new(command_db: Db, settings: Settings, info: AppInfo) -> Self {
        Self {
            cache: RwLock::new(Cache {
                dashboard: None,
                game: None,
                settings,
                info,
            }),
            command_db: Mutex::new(command_db),
            game_lock: Mutex::new(()),
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Cache> {
        self.cache.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Cache> {
        self.cache.write().unwrap_or_else(|e| e.into_inner())
    }

    /// The newest snapshot, `None` before the first one.
    pub fn dashboard(&self) -> Option<DashboardSnapshot> {
        self.read().dashboard.clone()
    }

    pub fn set_dashboard(&self, snapshot: DashboardSnapshot) {
        self.write().dashboard = Some(snapshot);
    }

    /// The newest game state, `None` before the first evaluation.
    pub fn game(&self) -> Option<GameState> {
        self.read().game.clone()
    }

    pub fn set_game(&self, state: GameState) {
        self.write().game = Some(state);
    }

    pub fn settings(&self) -> Settings {
        self.read().settings.clone()
    }

    pub fn set_settings(&self, settings: Settings) {
        self.write().settings = settings;
    }

    pub fn info(&self) -> AppInfo {
        self.read().info.clone()
    }

    pub fn set_info(&self, info: AppInfo) {
        self.write().info = info;
    }

    /// Runs `f` with the command connection.
    pub fn with_db<R>(&self, f: impl FnOnce(&Db) -> DbResult<R>) -> DbResult<R> {
        f(&lock(&self.command_db))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Mode, Platform, Season};

    pub(crate) fn info() -> AppInfo {
        AppInfo {
            version: "0.0.0".into(),
            platform: Platform::Windows,
            season: Season::Regular,
            schema_version: 1,
            logs_found: true,
        }
    }

    #[test]
    fn caches_start_empty_and_update() {
        let state = AppState::new(Db::open_in_memory().unwrap(), Settings::default(), info());
        assert!(state.dashboard().is_none() && state.game().is_none());
        assert_eq!(state.settings().mode, Mode::NoneYet);
        let s = Settings {
            mode: Mode::Max,
            ..Settings::default()
        };
        state.set_settings(s.clone());
        assert_eq!(state.settings(), s);
        let saved = state
            .with_db(|db| db.apply_settings_patch(&Default::default()))
            .unwrap();
        assert_eq!(saved.mode, Mode::NoneYet);
    }
}
