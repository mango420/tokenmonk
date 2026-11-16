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

use crate::app::{AppState, DbGameStore, IngestSupervisor, UpdateState};
use crate::contracts::events::{APP_INFO_CHANGED, LIMITS_CONNECTION_CHANGED, SETTINGS_CHANGED};
use crate::contracts::{
    AppInfo, ClaudeFolder, CommandError, CommandErrorCode, DashboardSnapshot, EpochMs, GameState,
    LimitSetup, LimitsConnection, Mode, Settings, SettingsPatch, UpdateStatus,
};
use crate::db::DbError;
use crate::game::{evaluate, Facts, GameStore};
use crate::limits::setup::{current_statusline_command, current_statusline_file};
use crate::limits::{bridge, connect};
use crate::watcher::{claude_folder, IngestConfig};

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
    if let Some(Some(path)) = &patch.claude_folder {
        if path.trim().is_empty() || !std::path::Path::new(path).is_absolute() {
            return Err(error(
                CommandErrorCode::InvalidArgument,
                "the Claude folder must be an absolute path",
            ));
        }
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
        if let Some(ingest) = app.try_state::<IngestSupervisor>() {
            ingest.recompute();
        }
    }
    if before.claude_folder != after.claude_folder {
        switch_claude_folder(app);
    }
}

/// Moves the ingest to the Claude folder in the settings and refreshes
/// `AppInfo`. The switch waits for the old worker, so it runs on its own
/// thread. The thread reads the settings itself once it holds the switch
/// lock, so the last change wins whatever order the threads run in.
fn switch_claude_folder(app: &AppHandle) {
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("claude-folder-switch".into())
        .spawn(move || {
            let Some(state) = app.try_state::<Arc<AppState>>() else {
                return;
            };
            let current = || state.settings().claude_folder;
            if let Some(ingest) = app.try_state::<IngestSupervisor>() {
                let result = ingest.switch(|| IngestConfig::for_setting(current().as_deref()));
                if let Err(e) = result {
                    log::warn!("could not switch the Claude folder, ingest is now off: {e}");
                }
            }
            // The status line setup lives in the folder in use.
            emit(&app, LIMITS_CONNECTION_CHANGED, &read_connection(&app));
            // Also after a failure: the folder in use changed either way.
            let folder = claude_folder(current().as_deref());
            let mut info = state.info();
            if info.logs_found != folder.projects_found {
                info.logs_found = folder.projects_found;
                state.set_info(info.clone());
                emit(&app, APP_INFO_CHANGED, &info);
            }
        });
    if let Err(e) = spawned {
        log::warn!("could not start the folder switch: {e}");
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
pub fn get_app_info(state: State<'_, Arc<AppState>>) -> Result<AppInfo, CommandError> {
    Ok(state.info())
}

#[tauri::command]
pub fn get_limit_setup() -> Result<LimitSetup, CommandError> {
    Ok(LimitSetup {
        statusline_command: current_statusline_command(),
        statusline_file: current_statusline_file(),
    })
}

fn connection_dirs(app: &AppHandle) -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
    let setting = app
        .try_state::<Arc<AppState>>()
        .and_then(|s| s.settings().claude_folder);
    (
        claude_folder(setting.as_deref()).path.map(Into::into),
        bridge::app_data_dir(),
    )
}

fn read_connection(app: &AppHandle) -> LimitsConnection {
    let (claude, data) = connection_dirs(app);
    match data {
        Some(data) => connect::inspect(claude.as_deref(), &data),
        None => connect::inspect(None, std::path::Path::new("")),
    }
}

fn claude_settings_error(e: connect::ConnectError) -> CommandError {
    error(CommandErrorCode::ClaudeSettings, e.to_string())
}

#[tauri::command]
pub fn get_limits_connection(app: AppHandle) -> Result<LimitsConnection, CommandError> {
    Ok(read_connection(&app))
}

#[tauri::command]
pub fn connect_limits(app: AppHandle) -> Result<LimitsConnection, CommandError> {
    let (claude, data) = connection_dirs(&app);
    let data = data.ok_or_else(|| error(CommandErrorCode::Internal, "no app data directory"))?;
    let command = current_statusline_command()
        .ok_or_else(|| error(CommandErrorCode::Internal, "no executable path"))?;
    let c = connect::connect(claude.as_deref(), &data, &command).map_err(claude_settings_error)?;
    emit(&app, LIMITS_CONNECTION_CHANGED, &c);
    Ok(c)
}

#[tauri::command]
pub fn disconnect_limits(app: AppHandle) -> Result<LimitsConnection, CommandError> {
    let (claude, data) = connection_dirs(&app);
    let data = data.ok_or_else(|| error(CommandErrorCode::Internal, "no app data directory"))?;
    let c = connect::disconnect(claude.as_deref(), &data).map_err(claude_settings_error)?;
    emit(&app, LIMITS_CONNECTION_CHANGED, &c);
    Ok(c)
}

#[tauri::command]
pub fn get_claude_folder(state: State<'_, Arc<AppState>>) -> Result<ClaudeFolder, CommandError> {
    Ok(claude_folder(state.settings().claude_folder.as_deref()))
}

#[tauri::command]
pub fn get_update_status(state: State<'_, UpdateState>) -> Result<UpdateStatus, CommandError> {
    Ok(state.status())
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateStatus, CommandError> {
    Ok(crate::app::check_update(&app).await)
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<UpdateStatus, CommandError> {
    crate::app::install_update(&app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Platform;
    use crate::db::Db;
    use chrono::FixedOffset;

    fn state(mode: Mode) -> AppState {
        let info = AppInfo {
            version: "0.0.0".into(),
            platform: Platform::Windows,
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
            update_check: None,
            keep_open: None,
            claude_folder: None,
        };
        let (before, after) = apply_update(&s, &patch, |_| Ok(())).unwrap();
        assert_eq!(before.mode, Mode::NoneYet);
        assert_eq!(after.mode, Mode::Diy);
        assert!(!after.sound);
        assert_eq!(s.settings(), after);
        assert_eq!(s.with_db(|db| db.settings()).unwrap(), after);
    }

    #[test]
    fn relative_or_empty_claude_folder_is_rejected() {
        let s = state(Mode::Max);
        for bad in ["", "  ", "relative/dir"] {
            let patch = SettingsPatch {
                claude_folder: Some(Some(bad.to_string())),
                ..SettingsPatch::default()
            };
            let err = apply_update(&s, &patch, |_| Ok(())).unwrap_err();
            assert_eq!(err.code, CommandErrorCode::InvalidArgument);
        }
        assert_eq!(s.settings().claude_folder, None);
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
    fn setup_command_is_provided() {
        let setup = get_limit_setup().unwrap();
        if let Some(cmd) = setup.statusline_command {
            assert!(cmd.ends_with(" --statusline"));
        }
    }
}
