//! User settings and onboarding.

use serde::{Deserialize, Deserializer, Serialize};

/// The play mode chosen in onboarding ("How do you want to vibe?").
///
/// `NoneYet` means onboarding has not been completed; the UI shows the
/// onboarding question instead of the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// `"noneYet"`: onboarding pending.
    #[default]
    NoneYet,
    /// `"max"`: Max it out (saffron). Use every last token.
    Max,
    /// `"diy"`: Do it yourself (jade). Give the AI as little as possible.
    Diy,
}

/// Persisted user settings. Returned by `get_settings`, `update_settings`,
/// `complete_onboarding` and the `settings-changed` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub mode: Mode,
    /// Play a sound on achievement unlocks and level-ups.
    pub sound: bool,
    /// Start Tokenmonk at login.
    pub autostart: bool,
    /// "Check for updates automatically": check once at app start. Manual
    /// checks ("Check now") work either way. Default on.
    pub update_check: bool,
    /// "Keep popup open": the popup no longer hides on blur and closes only
    /// via the tray icon or its Close button. Default off (hide on blur). Missing in older
    /// stored settings, so it defaults on deserialize.
    #[serde(default)]
    pub keep_open: bool,
    /// "Claude folder": the Claude config folder chosen in Settings (the
    /// app appends `projects/` itself). `None` = not set; the folder then
    /// comes from `CLAUDE_CONFIG_DIR` or the default `~/.claude`, see
    /// [`ClaudeFolder`](super::app::ClaudeFolder). Omitted in JSON when
    /// `None`; absent (older stored settings) or `null` reads as `None`.
    /// TS: `claudeFolder?: string | null`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_folder: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::NoneYet,
            sound: true,
            autostart: false,
            update_check: true,
            keep_open: false,
            claude_folder: None,
        }
    }
}

/// Partial update for `update_settings`. Absent fields stay unchanged.
/// Setting `mode` to `noneYet` is rejected with `invalidArgument`.
///
/// `claude_folder` is a double option: `None` (key absent) = unchanged,
/// `Some(None)` (JSON `null`) = reset to default, `Some(Some(path))` = set.
/// The path must be non-empty and absolute, otherwise `invalidArgument`;
/// whether it or its `projects/` exists is not checked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_open: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub claude_folder: Option<Option<String>>,
}

/// Maps a present key to `Some(..)`, so `null` becomes `Some(None)` and only
/// an absent key stays `None`.
fn present_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
