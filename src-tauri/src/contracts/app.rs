//! App-level information: version, platform, schema version and whether the
//! Claude Code logs were found. The UI reads it with `get_app_info` and
//! listens to `app-info-changed`.

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

/// Returned by `get_app_info` and pushed with `app-info-changed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// App version, for example "0.1.0".
    pub version: String,
    pub platform: Platform,
    /// Database schema version in use.
    pub schema_version: u32,
    /// Whether the Claude Code projects folder exists.
    pub logs_found: bool,
}
