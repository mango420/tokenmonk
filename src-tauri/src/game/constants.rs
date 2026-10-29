//! Every number and string of the game rules lives here: XP rules, the
//! level curve, titles per mode and achievement thresholds. Changing a value
//! changes the game; nothing else in the module hardcodes a rule.

use crate::contracts::{AchievementId, Mode};

// --- XP: Max it out ------------------------------------------------------

/// Headline tokens (input + output + cache creation) that make one XP block.
pub const TOKENS_PER_BLOCK: u64 = 100_000;
/// XP per completed block of tokens burned in a local day.
pub const XP_PER_BLOCK: u32 = 10;
/// Blocks that count per local day (2M tokens, 200 XP). Stops one extreme day
/// from skipping levels.
pub const MAX_BLOCKS_PER_DAY: u64 = 20;

// --- XP: Do it yourself --------------------------------------------------

/// XP for a finished day at 0 % of the daily budget. Falls linearly to 0 XP at
/// 100 % (23 % used is 77 XP, matching "+77 XP if you stop now").
pub const DIY_DAILY_MAX_XP: u32 = 100;
/// XP for a finished active day without a budget to judge it by (no limit
/// data and no token history yet).
pub const DIY_UNMEASURED_DAY_XP: u32 = 25;

// --- XP ledger kinds -----------------------------------------------------

/// Max it out: one row per token block, ref `"<day>/<block>"`.
pub const KIND_TOKENS: &str = "tokens";
/// Do it yourself: one row per finished day, ref `"<day>"` (`YYYY-MM-DD`).
pub const KIND_DAY: &str = "day";
/// Achievement bonus, ref is the achievement id string.
pub const KIND_ACHIEVEMENT: &str = "achievement";

// --- Level curve ---------------------------------------------------------

/// XP spanned by level `n` is `LEVEL_BASE_XP + LEVEL_STEP_XP * n`. With these
/// values level 4 spans 1,160 XP and level 7 spans 1,850 XP, the numbers of
/// the design mockups.
pub const LEVEL_BASE_XP: u64 = 240;
pub const LEVEL_STEP_XP: u64 = 230;

// --- Titles --------------------------------------------------------------

/// `(first level, title)`, ascending. A title holds until the next tier.
pub const TITLES_MAX: [(u32, &str); 7] = [
    (1, "Token Sprout"),
    (3, "Token Page"),
    (5, "Token Squire"),
    (9, "Rate-Limit Rambo"),
    (13, "Context Crusher"),
    (18, "Burn Baron"),
    (25, "Token Titan"),
];

pub const TITLES_DIY: [(u32, &str); 7] = [
    (1, "Curious Pilgrim"),
    (3, "Novice Monk"),
    (6, "Quiet Scribe"),
    (9, "Hand-Coding Hermit"),
    (13, "Calm Maker"),
    (18, "Silent Sage"),
    (25, "Grand Token Monk"),
];

// --- Achievements --------------------------------------------------------

/// Night Owl window, local time, start inclusive, end exclusive.
pub const NIGHT_START_HOUR: u32 = 2;
pub const NIGHT_END_HOUR: u32 = 4;

/// Rate-Limit Rambo: a 5-hour reading at or above this percentage.
pub const RAMBO_PERCENT: f64 = 100.0;
/// So Close: a finished 5-hour window whose peak is at least this and below
/// `RAMBO_PERCENT`.
pub const SO_CLOSE_PERCENT: f64 = 99.0;
/// Readings whose 5-hour reset times are within one bucket belong to one
/// window (the reset timestamp jitters by milliseconds between readings).
pub const WINDOW_BUCKET_MS: i64 = 60_000;

/// Token Monk: a finished day counts when its budget usage is below this
/// percentage ...
pub const TOKEN_MONK_PERCENT: f64 = 20.0;
/// ... and it takes this many such days in a row.
pub const TOKEN_MONK_DAYS: u32 = 7;
/// Calendar days the caller supplies per evaluation so a streak can be found:
/// the streak target plus the days a missed evaluation can skip.
pub const STREAK_LOOKBACK_DAYS: u32 = 14;

/// Newest unlocks carried in `GameState::recent_unlocks`.
pub const RECENT_UNLOCKS: usize = 5;

/// Static definition of an achievement. Display order is the order of
/// [`ACHIEVEMENTS`].
#[derive(Debug, Clone, Copy)]
pub struct AchievementDef {
    pub id: AchievementId,
    pub name: &'static str,
    pub description: &'static str,
    pub secret: bool,
    /// `None`: both modes.
    pub mode: Option<Mode>,
    pub xp: u32,
    /// Counted goal, `None` for one-shots.
    pub target: Option<u32>,
}

pub const ACHIEVEMENTS: [AchievementDef; 6] = [
    AchievementDef {
        id: AchievementId::FirstBlood,
        name: "First Blood",
        description: "Burn your first tokens.",
        secret: false,
        mode: None,
        xp: 25,
        target: None,
    },
    AchievementDef {
        id: AchievementId::NightOwl,
        name: "Night Owl",
        description: "Burn tokens between 02:00 and 04:00.",
        secret: false,
        mode: Some(Mode::Max),
        xp: 50,
        target: None,
    },
    AchievementDef {
        id: AchievementId::RateLimitRambo,
        name: "Rate-Limit Rambo",
        description: "Hit the 5-hour limit.",
        secret: false,
        mode: Some(Mode::Max),
        xp: 100,
        target: None,
    },
    AchievementDef {
        id: AchievementId::SoClose,
        name: "So Close",
        description: "Stop at 99 % of the 5-hour limit.",
        secret: false,
        mode: Some(Mode::Max),
        xp: 100,
        target: None,
    },
    AchievementDef {
        id: AchievementId::TokenMonk,
        name: "Token Monk",
        description: "Stay under 20 % of your daily budget for 7 days in a row.",
        secret: false,
        mode: Some(Mode::Diy),
        xp: 150,
        target: Some(TOKEN_MONK_DAYS),
    },
    AchievementDef {
        id: AchievementId::Boo,
        name: "Boo",
        description: "Find the ghost.",
        secret: true,
        mode: None,
        xp: 100,
        target: None,
    },
];

/// Definition of one achievement.
pub fn achievement_def(id: AchievementId) -> &'static AchievementDef {
    ACHIEVEMENTS
        .iter()
        .find(|d| d.id == id)
        .expect("every AchievementId has a definition")
}
