//! Tauri command names, their argument shapes and the shared error type.
//!
//! | Command               | Arguments                 | Returns            |
//! |-----------------------|---------------------------|--------------------|
//! | `get_dashboard`       | none                      | `DashboardSnapshot`|
//! | `get_settings`        | none                      | `Settings`         |
//! | `update_settings`     | `UpdateSettingsArgs`      | `Settings`         |
//! | `complete_onboarding` | `CompleteOnboardingArgs`  | `Settings`         |
//! | `get_game_state`      | none                      | `GameState`        |
//! | `trigger_easter_egg`  | none                      | `EasterEggResult`  |
//! | `get_app_info`        | none                      | `AppInfo`          |
//!
//! Every command returns `Result<T, CommandError>`; on the TS side a failed
//! `invoke` rejects with a `CommandError` object.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::settings::{Mode, SettingsPatch};

pub const GET_DASHBOARD: &str = "get_dashboard";
pub const GET_SETTINGS: &str = "get_settings";
pub const UPDATE_SETTINGS: &str = "update_settings";
pub const COMPLETE_ONBOARDING: &str = "complete_onboarding";
pub const GET_GAME_STATE: &str = "get_game_state";
pub const TRIGGER_EASTER_EGG: &str = "trigger_easter_egg";
pub const GET_APP_INFO: &str = "get_app_info";

/// Every command name, for registration checks in tests.
pub const ALL: [&str; 7] = [
    GET_DASHBOARD,
    GET_SETTINGS,
    UPDATE_SETTINGS,
    COMPLETE_ONBOARDING,
    GET_GAME_STATE,
    TRIGGER_EASTER_EGG,
    GET_APP_INFO,
];

/// Arguments of `update_settings`: `{ "patch": { ... } }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsArgs {
    pub patch: SettingsPatch,
}

/// Arguments of `complete_onboarding`: `{ "mode": "max" | "diy" }`.
/// `noneYet` is rejected with `invalidArgument`. Completing onboarding also
/// starts the retroactive levelling replay if it has not run yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteOnboardingArgs {
    pub mode: Mode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandErrorCode {
    /// `"invalidArgument"`: the arguments were rejected.
    InvalidArgument,
    /// `"notReady"`: the core is still starting (database not open yet).
    NotReady,
    /// `"storage"`: the database failed.
    Storage,
    /// `"internal"`: anything else.
    Internal,
}

/// Error payload of every command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: CommandErrorCode,
    /// Technical message for logs; the UI shows its own calm copy.
    pub message: String,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}
