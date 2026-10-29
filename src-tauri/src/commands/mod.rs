//! Tauri command handlers exposed to the frontend.
//!
//! These are registration stubs: each one matches its contract in
//! `contracts::commands` and returns a "not implemented" error until the
//! owning slice replaces the body. Argument names (`patch`, `mode`) are the
//! wire names from the contracts.

use crate::contracts::{
    AppInfo, CommandError, CommandErrorCode, DashboardSnapshot, EasterEggResult, GameState, Mode,
    Settings, SettingsPatch,
};

fn not_implemented(command: &str) -> CommandError {
    CommandError {
        code: CommandErrorCode::Internal,
        message: format!("{command}: not implemented"),
    }
}

#[tauri::command]
pub fn get_dashboard() -> Result<DashboardSnapshot, CommandError> {
    Err(not_implemented("get_dashboard"))
}

#[tauri::command]
pub fn get_settings() -> Result<Settings, CommandError> {
    Err(not_implemented("get_settings"))
}

#[tauri::command]
#[allow(unused_variables)]
pub fn update_settings(patch: SettingsPatch) -> Result<Settings, CommandError> {
    Err(not_implemented("update_settings"))
}

#[tauri::command]
#[allow(unused_variables)]
pub fn complete_onboarding(mode: Mode) -> Result<Settings, CommandError> {
    Err(not_implemented("complete_onboarding"))
}

#[tauri::command]
pub fn get_game_state() -> Result<GameState, CommandError> {
    Err(not_implemented("get_game_state"))
}

#[tauri::command]
pub fn trigger_easter_egg() -> Result<EasterEggResult, CommandError> {
    Err(not_implemented("trigger_easter_egg"))
}

#[tauri::command]
pub fn get_app_info() -> Result<AppInfo, CommandError> {
    Err(not_implemented("get_app_info"))
}
