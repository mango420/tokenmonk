//! User settings and onboarding.

use serde::{Deserialize, Deserializer, Serialize};

/// The play mode.
///
/// `NoneYet` means onboarding has not been completed. Since TKM-287 Max is
/// the only mode: `Diy` is removed (see the variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// `"noneYet"`: onboarding pending.
    #[default]
    NoneYet,
    /// `"max"`: Max it out (saffron). Use every last token.
    Max,
    /// `"diy"`: Do it yourself. Removed by TKM-287: kept only until
    /// TKM-301/302/306/307 stop using it (cleanup in TKM-300). Stored `diy`
    /// is migrated to `max` silently (TKM-301); XP and levels stay. Never
    /// produce it in new code.
    Diy,
}

/// Persisted user settings. Returned by `get_settings`, `update_settings`,
/// `complete_onboarding` and the `settings-changed` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub mode: Mode,
    /// Play the in-app tone on achievement unlocks and level-ups. Does not
    /// apply to system notifications; they use the OS sound.
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
    /// "Limits at 10 % left": one system notification per limit window and
    /// reset cycle when the window drops to 10 % left (docs/ARCHITECTURE.md,
    /// "Notifications"). Default on; absent on input reads as on.
    #[serde(default = "default_true")]
    pub notify_limits: bool,
    /// "Level-ups": a system notification per live level-up. Default on;
    /// absent on input reads as on.
    #[serde(default = "default_true")]
    pub notify_level_ups: bool,
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
            notify_limits: true,
            notify_level_ups: true,
            claude_folder: None,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Partial update for `update_settings`. Absent fields stay unchanged.
/// Setting `mode` to `noneYet` is rejected with `invalidArgument`.
///
/// `mode`: removed by TKM-287 (no mode picker); kept until TKM-307 stops
/// using it (cleanup in TKM-300).
///
/// `notifyLimits` and `notifyLevelUps` (`Option<bool>`, matching the two
/// `Settings` fields) are added in the TKM-300 cleanup: a test in `db`
/// builds this struct with every field listed, so a new field would break
/// it until TKM-301 rewrites that test.
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
