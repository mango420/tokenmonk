//! Gamification types. Only shapes and identifiers live here; XP rules, the
//! level curve, titles and achievement thresholds belong to the `game`
//! module. Display strings for game content (titles, achievement names and
//! progress lines) are produced by the game module in English.

use serde::{Deserialize, Serialize};

use super::settings::Mode;
use super::EpochMs;

/// Achievement identifiers. Stored as these strings in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AchievementId {
    /// `"firstBlood"`
    FirstBlood,
    /// `"nightOwl"`
    NightOwl,
    /// `"rateLimitRambo"`
    RateLimitRambo,
    /// `"soClose"`
    SoClose,
    /// `"tokenMonk"`
    TokenMonk,
    /// `"boo"`: secret, unlocked by the logo easter egg.
    Boo,
}

impl AchievementId {
    pub const ALL: [AchievementId; 6] = [
        AchievementId::FirstBlood,
        AchievementId::NightOwl,
        AchievementId::RateLimitRambo,
        AchievementId::SoClose,
        AchievementId::TokenMonk,
        AchievementId::Boo,
    ];

    /// The wire and database string of this id.
    pub fn as_str(self) -> &'static str {
        match self {
            AchievementId::FirstBlood => "firstBlood",
            AchievementId::NightOwl => "nightOwl",
            AchievementId::RateLimitRambo => "rateLimitRambo",
            AchievementId::SoClose => "soClose",
            AchievementId::TokenMonk => "tokenMonk",
            AchievementId::Boo => "boo",
        }
    }
}

/// Definition plus progress of one achievement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementProgress {
    pub id: AchievementId,
    /// Display name, for example "Token Monk".
    pub name: String,
    /// What it takes, one short sentence.
    pub description: String,
    /// Hidden until unlocked (Boo).
    pub secret: bool,
    /// `Some(mode)` if it can only be earned in that mode, `None` for both.
    pub mode: Option<Mode>,
    /// XP granted on unlock.
    pub xp: u32,
    pub unlocked_at: Option<EpochMs>,
    /// Progress towards a counted goal, for example 4 (days).
    pub current: Option<u32>,
    /// The counted goal, for example 7 (days). `None` for one-shot achievements.
    pub target: Option<u32>,
    /// Ready-made progress line, for example "4 of 7 days under 20 %".
    pub progress_label: Option<String>,
}

/// Payload of the `achievement-unlocked` event; also listed in
/// `GameState::recent_unlocks`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementUnlocked {
    pub id: AchievementId,
    pub name: String,
    /// Context line, for example "Tokens burned at 02:47".
    pub detail: Option<String>,
    pub xp: u32,
    pub unlocked_at: EpochMs,
    /// Unlocked by the first-start history replay. Retroactive unlocks are
    /// not toasted one by one.
    pub retroactive: bool,
}

/// Payload of the `level-up` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelUp {
    pub level: u32,
    pub previous_level: u32,
    /// Title at the new level (titles depend on mode and level).
    pub title: String,
    pub title_changed: bool,
    pub retroactive: bool,
}

/// Full game state. Returned by `get_game_state` and pushed with
/// `game-updated`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub mode: Mode,
    pub level: u32,
    /// For example "Token Squire".
    pub title: String,
    /// All XP ever earned (one pool for both modes).
    pub xp_total: u64,
    /// XP earned inside the current level.
    pub xp_into_level: u64,
    /// XP the current level spans; `xp_into_level / xp_for_level` is the bar.
    pub xp_for_level: u64,
    /// `xp_into_level / xp_for_level`, `0.0..=1.0`.
    pub progress: f64,
    /// Do it yourself only: XP today's usage would earn if it stopped now
    /// ("+77 XP if you stop now"). `Some(0)` when over budget, `None` in
    /// Max it out or without a daily budget.
    pub daily_xp_preview: Option<u32>,
    /// Newest first, at most a handful.
    pub recent_unlocks: Vec<AchievementUnlocked>,
    /// Every achievement, in display order.
    pub achievements: Vec<AchievementProgress>,
    /// The first-start history replay has finished.
    pub retro_done: bool,
}

/// Result of `trigger_easter_egg`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EasterEggResult {
    /// `true` the first time; later triggers still show the ghost.
    pub newly_unlocked: bool,
    /// The Boo achievement (with its original unlock time on repeats).
    pub achievement: AchievementUnlocked,
}
