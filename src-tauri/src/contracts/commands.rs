//! Tauri command names, their argument shapes and the shared error type.
//!
//! | Command               | Arguments                 | Returns            |
//! |-----------------------|---------------------------|--------------------|
//! | `get_dashboard`       | none                      | `DashboardSnapshot`|
//! | `get_settings`        | none                      | `Settings`         |
//! | `update_settings`     | `UpdateSettingsArgs`      | `Settings`         |
//! | `complete_onboarding` | `CompleteOnboardingArgs`  | `Settings`         |
//! | `get_game_state`      | none                      | `GameState`        |
//! | `get_app_info`        | none                      | `AppInfo`          |
//! | `get_limit_setup`     | none                      | `LimitSetup`       |
//! | `get_update_status`   | none                      | `UpdateStatus`     |
//! | `check_for_updates`   | none                      | `UpdateStatus`     |
//! | `install_update`      | none                      | `UpdateStatus`     |
//!
//! `check_for_updates` resolves when the check is done (or at once with the
//! current status while a check or install is running). `install_update`
//! resolves only on failure (`installFailed`): on success the app exits or
//! restarts. Without an installable version it rejects with
//! `invalidArgument`. Update failures are statuses, never `CommandError`s.
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
pub const GET_APP_INFO: &str = "get_app_info";
pub const GET_LIMIT_SETUP: &str = "get_limit_setup";
pub const GET_UPDATE_STATUS: &str = "get_update_status";
pub const CHECK_FOR_UPDATES: &str = "check_for_updates";
pub const INSTALL_UPDATE: &str = "install_update";

/// Every command name, for registration checks in tests.
pub const ALL: [&str; 10] = [
    GET_DASHBOARD,
    GET_SETTINGS,
    UPDATE_SETTINGS,
    COMPLETE_ONBOARDING,
    GET_GAME_STATE,
    GET_APP_INFO,
    GET_LIMIT_SETUP,
    GET_UPDATE_STATUS,
    CHECK_FOR_UPDATES,
    INSTALL_UPDATE,
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
