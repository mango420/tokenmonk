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
//! | `get_claude_folder`   | none                      | `ClaudeFolder`     |
//! | `get_limits_connection` | none                    | `LimitsConnection` |
//! | `connect_limits`      | none                      | `LimitsConnection` |
//! | `disconnect_limits`   | none                      | `LimitsConnection` |
//!
//! `check_for_updates` resolves when the check is done (or at once with the
//! current status while a check or install is running). `install_update`
//! resolves only on failure (`installFailed`): on success the app exits or
//! restarts. Without an installable version it rejects with
//! `invalidArgument`. Update failures are statuses, never `CommandError`s.
//! `get_claude_folder` has no event of its own: the UI calls it again after
//! `settings-changed` and `app-info-changed`.
//!
//! `connect_limits` and `disconnect_limits` run only on an explicit click in
//! Settings and are idempotent (connect while connected rewrites the entry
//! with the current bridge command; disconnect while not connected changes
//! nothing). Both reject with `claudeSettings` if the state is `blocked` or
//! the write fails; then `settings.json` is unchanged. On success they also
//! emit `limits-connection-changed`. Rules: docs/ARCHITECTURE.md, "Connect
//! limits".
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
pub const GET_CLAUDE_FOLDER: &str = "get_claude_folder";
pub const GET_LIMITS_CONNECTION: &str = "get_limits_connection";
pub const CONNECT_LIMITS: &str = "connect_limits";
pub const DISCONNECT_LIMITS: &str = "disconnect_limits";

/// Every command name, for registration checks in tests.
pub const ALL: [&str; 14] = [
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
    GET_CLAUDE_FOLDER,
    GET_LIMITS_CONNECTION,
    CONNECT_LIMITS,
    DISCONNECT_LIMITS,
];

/// Arguments of `update_settings`: `{ "patch": { ... } }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsArgs {
    pub patch: SettingsPatch,
}

/// Arguments of `complete_onboarding`: `{ "mode": "max" }`.
/// `noneYet` is rejected with `invalidArgument`. `diy` is removed by
/// TKM-287 and accepted only until TKM-301 and TKM-306 drop it. Completing onboarding also
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
    /// `"claudeSettings"`: `connect_limits` or `disconnect_limits` could not
    /// use `settings.json` in the Claude folder (blocked, or the backup or
    /// write failed). The file is unchanged.
    ClaudeSettings,
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
