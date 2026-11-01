//! Event names (Rust -> UI) and the payloads that have no other home.
//!
//! | Event                  | Payload               | When                                   |
//! |------------------------|-----------------------|----------------------------------------|
//! | `dashboard-updated`    | `DashboardSnapshot`   | after new log lines, limit readings or the minute tick |
//! | `settings-changed`     | `Settings`            | after any settings change              |
//! | `game-updated`         | `GameState`           | after XP, level or progress changes    |
//! | `achievement-unlocked` | `AchievementUnlocked` | once per live (non-retroactive) unlock |
//! | `level-up`             | `LevelUp`             | once per live level change             |
//! | `app-info-changed`     | `AppInfo`             | when `logsFound` changes               |
//! | `navigate`             | `NavigateTo`          | tray menu asks the popup to show a view|
//!
//! All events are emitted to every window. Payloads are full state, never
//! diffs, so a missed event is healed by the next one.

use serde::{Deserialize, Serialize};

pub const DASHBOARD_UPDATED: &str = "dashboard-updated";
pub const SETTINGS_CHANGED: &str = "settings-changed";
pub const GAME_UPDATED: &str = "game-updated";
pub const ACHIEVEMENT_UNLOCKED: &str = "achievement-unlocked";
pub const LEVEL_UP: &str = "level-up";
pub const APP_INFO_CHANGED: &str = "app-info-changed";
pub const NAVIGATE: &str = "navigate";

/// Every event name, for tests.
pub const ALL: [&str; 7] = [
    DASHBOARD_UPDATED,
    SETTINGS_CHANGED,
    GAME_UPDATED,
    ACHIEVEMENT_UNLOCKED,
    LEVEL_UP,
    APP_INFO_CHANGED,
    NAVIGATE,
];

/// Views of the popup window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum View {
    /// `"dashboard"`
    Dashboard,
    /// `"settings"`
    Settings,
    /// `"onboarding"`
    Onboarding,
}

/// Payload of `navigate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateTo {
    pub view: View,
}
