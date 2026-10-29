//! App-level information, the season and the easter egg constants.
//!
//! The season is computed in Rust (local time) because the tray icon, which
//! Rust renders, needs it too. The UI never computes it; it reads
//! `AppInfo::season` and listens to `app-info-changed`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    /// `"macos"`
    Macos,
    /// `"windows"`
    Windows,
    /// `"linux"`: not a v0.1 target, development only.
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Season {
    /// `"regular"`
    #[default]
    Regular,
    /// `"halloween"`: 24 Oct 00:00 to 2 Nov 23:59 local time, both days included.
    Halloween,
}

/// Halloween start, (month, day), inclusive, local time.
pub const HALLOWEEN_START: (u32, u32) = (10, 24);
/// Halloween end, (month, day), inclusive, local time.
pub const HALLOWEEN_END: (u32, u32) = (11, 2);

/// Logo clicks needed for the ghost easter egg.
pub const EASTER_EGG_CLICKS: u32 = 3;
/// Maximum gap between two counted logo clicks, in milliseconds.
pub const EASTER_EGG_WINDOW_MS: u32 = 800;

/// Returned by `get_app_info` and pushed with `app-info-changed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// App version, for example "0.1.0".
    pub version: String,
    pub platform: Platform,
    pub season: Season,
    /// Database schema version in use.
    pub schema_version: u32,
    /// Whether the Claude Code projects folder exists.
    pub logs_found: bool,
}
