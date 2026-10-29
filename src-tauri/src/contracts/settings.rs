//! User settings and onboarding.

use serde::{Deserialize, Serialize};

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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::NoneYet,
            sound: true,
            autostart: false,
        }
    }
}

/// Partial update for `update_settings`. Absent fields stay unchanged.
/// Setting `mode` to `noneYet` is rejected with `invalidArgument`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autostart: Option<bool>,
}
