//! Tauri command handlers exposed to the frontend (docs/ARCHITECTURE.md,
//! "Commands and events").
//!
//! Handlers read the cache in [`AppState`]; they never parse logs. Writes go
//! through the command connection. The testable logic sits in plain
//! functions below the handlers; the handlers only add Tauri plumbing
//! (events, autostart plugin, ingest worker). Argument names (`patch`,
//! `mode`) are the wire names from the contracts.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Local, TimeZone};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::app::{AppState, DbGameStore};
use crate::contracts::events::{ACHIEVEMENT_UNLOCKED, GAME_UPDATED, SETTINGS_CHANGED};
use crate::contracts::{
    AppInfo, CommandError, CommandErrorCode, DashboardSnapshot, EasterEggResult, EpochMs,
    GameState, LimitSetup, Mode, Settings, SettingsPatch,
};
use crate::db::DbError;
use crate::game::{easter_egg, evaluate, persist, Facts, GameStore};
use crate::limits::setup::current_statusline_command;
use crate::watcher::IngestHandle;

fn error(code: CommandErrorCode, message: impl Into<String>) -> CommandError {
    CommandError {
        code,
        message: message.into(),
    }
}

fn storage(e: DbError) -> CommandError {
    error(CommandErrorCode::Storage, e.to_string())
}

fn now_ms() -> EpochMs {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Validates and stores a settings patch. `set_autostart` is called only if
/// the autostart value actually changes; if it fails nothing is stored.
/// Returns `(before, after)`; the cache holds `after`.
pub(crate) fn apply_update(
    state: &AppState,
    patch: &SettingsPatch,
    set_autostart: impl FnOnce(bool) -> Result<(), String>,
) -> Result<(Settings, Settings), CommandError> {
    if patch.mode == Some(Mode::NoneYet) {
        return Err(error(
            CommandErrorCode::InvalidArgument,
            "mode cannot be set back to noneYet",
        ));
    }
    let before = state.settings();
    if let Some(wanted) = patch.autostart {
        if wanted != before.autostart {
            set_autostart(wanted).map_err(|e| error(CommandErrorCode::Internal, e))?;
        }
    }
    let after = state
        .with_db(|db| db.apply_settings_patch(patch))
        .map_err(storage)?;
    state.set_settings(after.clone());
    Ok((before, after))
}

/// Mode of onboarding: `noneYet` is rejected.
pub(crate) fn onboarding_patch(mode: Mode) -> Result<SettingsPatch, CommandError> {
    if mode == Mode::NoneYet {
        return Err(error(
            CommandErrorCode::InvalidArgument,
            "choose max or diy",
        ));
    }
    Ok(SettingsPatch {
        mode: Some(mode),
        ..SettingsPatch::default()
    })
}

/// The cached game state; before the first evaluation it is derived from
/// the stored ledger without writing anything.
pub(crate) fn game_state<Tz: TimeZone>(
    state: &AppState,
    now: EpochMs,
    tz: &Tz,
) -> Result<GameState, CommandError> {
    if let Some(game) = state.game() {
        return Ok(game);
    }
    let mode = state.settings().mode;
    let data = state
        .with_db(|db| DbGameStore::new(db).load())
        .map_err(storage)?;
    Ok(evaluate(&data, &Facts::default(), mode, now, tz, false).state)
}

/// The ghost easter egg. Returns the result and, when Boo was newly
/// unlocked, the game state to announce.
pub(crate) fn run_easter_egg<Tz: TimeZone>(
    state: &AppState,
    now: EpochMs,
    tz: &Tz,
) -> Result<(EasterEggResult, Option<GameState>), CommandError> {
    let _guard = state.game_lock.lock().unwrap_or_else(|e| e.into_inner());
    let mode = state.settings().mode;
    let (result, evaluation) = state
        .with_db(|db| {
            let mut store = DbGameStore::new(db);
            let prev = store.load()?;
            let (result, evaluation) = easter_egg(&prev, mode, now, tz);
            persist(&mut store, &prev, &evaluation, false)?;
            Ok((result, evaluation))
        })
        .map_err(storage)?;
    if !result.newly_unlocked {
        return Ok((result, None));
    }
    let mut game = evaluation.state;
    // The evaluation has no day facts: keep the preview the worker computed.
    game.daily_xp_preview = state.game().and_then(|g| g.daily_xp_preview);
    state.set_game(game.clone());
    Ok((result, Some(game)))
}

fn emit<T: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: &T) {
    if let Err(e) = app.emit(event, payload) {
        log::warn!("could not emit {event}: {e}");
    }
}

#[cfg(desktop)]
fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| format!("autostart: {e}"))
}

#[cfg(not(desktop))]
fn set_autostart(_app: &AppHandle, _enabled: bool) -> Result<(), String> {
    Ok(())
}

/// Announces new settings and lets the worker redo what depends on the mode.
fn settings_applied(app: &AppHandle, before: &Settings, after: &Settings, force_recompute: bool) {
    if before != after {
        emit(app, SETTINGS_CHANGED, after);
    }
    if force_recompute || before.mode != after.mode {
        if let Some(ingest) = app.try_state::<IngestHandle>() {
            ingest.recompute();
        }
    }
}

#[tauri::command]
pub fn get_dashboard(state: State<'_, Arc<AppState>>) -> Result<DashboardSnapshot, CommandError> {
    state.dashboard().ok_or_else(|| {
        error(
            CommandErrorCode::NotReady,
            "the first snapshot is not ready yet",
        )
    })
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, CommandError> {
    Ok(state.settings())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    patch: SettingsPatch,
) -> Result<Settings, CommandError> {
    let (before, after) = apply_update(&state, &patch, |on| set_autostart(&app, on))?;
    settings_applied(&app, &before, &after, false);
    Ok(after)
}

#[tauri::command]
pub fn complete_onboarding(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    mode: Mode,
) -> Result<Settings, CommandError> {
    let patch = onboarding_patch(mode)?;
    let (before, after) = apply_update(&state, &patch, |on| set_autostart(&app, on))?;
    // Starts the retroactive replay if it has not run yet.
    settings_applied(&app, &before, &after, true);
    Ok(after)
}

#[tauri::command]
pub fn get_game_state(state: State<'_, Arc<AppState>>) -> Result<GameState, CommandError> {
    game_state(&state, now_ms(), &Local)
}

#[tauri::command]
pub fn trigger_easter_egg(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<EasterEggResult, CommandError> {
    let (result, announce) = run_easter_egg(&state, now_ms(), &Local)?;
    if let Some(game) = announce {
        emit(&app, ACHIEVEMENT_UNLOCKED, &result.achievement);
        emit(&app, GAME_UPDATED, &game);
    }
    Ok(result)
}

#[tauri::command]
pub fn get_app_info(state: State<'_, Arc<AppState>>) -> Result<AppInfo, CommandError> {
    Ok(state.info())
}

#[tauri::command]
pub fn get_limit_setup() -> Result<LimitSetup, CommandError> {
    Ok(LimitSetup {
        statusline_command: current_statusline_command(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{AchievementId, Platform, Season};
    use crate::db::Db;
    use chrono::FixedOffset;

    fn state(mode: Mode) -> AppState {
        let info = AppInfo {
            version: "0.0.0".into(),
            platform: Platform::Windows,
            season: Season::Regular,
            schema_version: 1,
            logs_found: true,
        };
        let db = Db::open_in_memory().unwrap();
        db.apply_settings_patch(&SettingsPatch {
            mode: Some(mode),
            ..SettingsPatch::default()
        })
        .ok();
        let settings = db.settings().unwrap();
        AppState::new(db, settings, info)
    }

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    #[test]
    fn noneyet_is_rejected_everywhere() {
        let s = state(Mode::Max);
        let patch = SettingsPatch {
            mode: Some(Mode::NoneYet),
            ..SettingsPatch::default()
        };
        let err = apply_update(&s, &patch, |_| Ok(())).unwrap_err();
        assert_eq!(err.code, CommandErrorCode::InvalidArgument);
        assert_eq!(s.settings().mode, Mode::Max);
        assert_eq!(
            onboarding_patch(Mode::NoneYet).unwrap_err().code,
            CommandErrorCode::InvalidArgument
        );
        assert_eq!(onboarding_patch(Mode::Diy).unwrap().mode, Some(Mode::Diy));
    }

    #[test]
    fn update_persists_and_updates_the_cache() {
        let s = state(Mode::NoneYet);
        let patch = SettingsPatch {
            mode: Some(Mode::Diy),
            sound: Some(false),
            autostart: None,
        };
        let (before, after) = apply_update(&s, &patch, |_| Ok(())).unwrap();
        assert_eq!(before.mode, Mode::NoneYet);
        assert_eq!(after.mode, Mode::Diy);
        assert!(!after.sound);
        assert_eq!(s.settings(), after);
        assert_eq!(s.with_db(|db| db.settings()).unwrap(), after);
    }

    #[test]
    fn autostart_is_toggled_only_on_change_and_failure_stores_nothing() {
        let s = state(Mode::Max);
        let on = SettingsPatch {
            autostart: Some(true),
            ..SettingsPatch::default()
        };
        let mut calls = Vec::new();
        let (_, after) = apply_update(&s, &on, |v| {
            calls.push(v);
            Ok(())
        })
        .unwrap();
        assert!(after.autostart);
        // Same value again: the plugin is not touched.
        apply_update(&s, &on, |v| {
            calls.push(v);
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, [true]);

        let off = SettingsPatch {
            autostart: Some(false),
            ..SettingsPatch::default()
        };
        let err = apply_update(&s, &off, |_| Err("denied".into())).unwrap_err();
        assert_eq!(err.code, CommandErrorCode::Internal);
        assert!(s.settings().autostart);
        assert!(s.with_db(|db| db.settings()).unwrap().autostart);
    }

    #[test]
    fn game_state_is_derived_before_the_first_evaluation() {
        let s = state(Mode::Max);
        let g = game_state(&s, 1_793_232_000_000, &tz()).unwrap();
        assert_eq!(g.xp_total, 0);
        assert_eq!(g.mode, Mode::Max);
        assert!(!g.retro_done);
        // Nothing was written.
        assert!(s.with_db(|db| db.xp_events()).unwrap().is_empty());
    }

    #[test]
    fn easter_egg_unlocks_boo_once() {
        let s = state(Mode::Max);
        let now = 1_793_232_000_000;
        let (first, announce) = run_easter_egg(&s, now, &tz()).unwrap();
        assert!(first.newly_unlocked);
        assert_eq!(first.achievement.id, AchievementId::Boo);
        let game = announce.expect("announced");
        assert_eq!(s.game().unwrap(), game);

        let (again, announce) = run_easter_egg(&s, now + 5_000, &tz()).unwrap();
        assert!(!again.newly_unlocked);
        assert!(announce.is_none());
        assert_eq!(again.achievement.unlocked_at, first.achievement.unlocked_at);
    }

    #[test]
    fn setup_command_is_provided() {
        let setup = get_limit_setup().unwrap();
        if let Some(cmd) = setup.statusline_command {
            assert!(cmd.ends_with(" --statusline"));
        }
    }
}
