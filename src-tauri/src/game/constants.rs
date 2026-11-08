//! Every number and string of the game rules lives here: XP rules, the
//! level curve, titles per mode and achievement thresholds. Changing a value
//! changes the game; nothing else in the module hardcodes a rule.

use crate::contracts::{AchievementId, Mode};

// --- Rules version -------------------------------------------------------

/// Version of the XP rules and the level curve. A ledger built with an older
/// version is recounted once (see `recount_if_outdated`). Bump it whenever a
/// rule below changes what an already-stored day would earn.
pub const RULES_VERSION: u32 = 2;

// --- XP: Max it out ------------------------------------------------------

/// Headline tokens (input + output + cache creation) that make one XP block.
pub const TOKENS_PER_BLOCK: u64 = 100_000;
/// Blocks per local day paid at `XP_PER_BLOCK` (0 to 2M tokens, 200 XP).
pub const FULL_RATE_BLOCKS_PER_DAY: u64 = 20;
/// XP per block inside the full-rate band.
pub const XP_PER_BLOCK: u32 = 10;
/// Last block of the soft band (2M to 5M tokens). Blocks above pay nothing.
pub const SOFT_BAND_END_BLOCK: u64 = 50;
/// XP per block inside the soft band (60 XP at most).
pub const XP_PER_SOFT_BLOCK: u32 = 2;
/// XP for every local day with at least one counted reply.
pub const MAX_ACTIVE_DAY_XP: u32 = 10;

// --- XP: Do it yourself --------------------------------------------------

/// Budget used up to this percentage earns `DIY_DAILY_MAX_XP`.
pub const DIY_FULL_XP_UP_TO_PERCENT: f64 = 50.0;
/// XP for a finished day at or below `DIY_FULL_XP_UP_TO_PERCENT`.
pub const DIY_DAILY_MAX_XP: u32 = 150;
/// XP for a finished day at exactly 100 % of the budget; falls linearly from
/// `DIY_DAILY_MAX_XP` at 50 %. Above 100 % a day earns 0.
pub const DIY_DAILY_MIN_XP: u32 = 30;
/// XP for a finished active day without a budget to judge it by (no limit
/// data and no token history yet).
pub const DIY_UNMEASURED_DAY_XP: u32 = 50;

// --- XP ledger kinds -----------------------------------------------------

/// Max it out: one row per token block, ref `"<day>/<block>"`.
pub const KIND_TOKENS: &str = "tokens";
/// Per local day, ref `"<day>"` (`YYYY-MM-DD`): the Do it yourself day award
/// or the Max it out active-day bonus.
pub const KIND_DAY: &str = "day";
/// Achievement bonus, ref is the achievement id string.
pub const KIND_ACHIEVEMENT: &str = "achievement";
/// Zero-XP marker written by the recount: the mode a day was earned in, ref
/// `"<day>"`. Keeps a day with the mode that earned it after its XP rows are
/// dropped.
pub const KIND_DAY_MODE: &str = "daymode";
/// Ledger kinds the recount drops.
pub const RECOUNT_DROP_KINDS: [&str; 2] = [KIND_TOKENS, KIND_DAY];

// --- Level curve ---------------------------------------------------------

/// XP spanned by level 1. Level `n` spans `LEVEL_FIRST_SPAN_XP +
/// LEVEL_STEP_XP * (n - 1)` up to `LEVEL_FLAT_FROM - 1`.
pub const LEVEL_FIRST_SPAN_XP: u64 = 200;
pub const LEVEL_STEP_XP: u64 = 120;
/// From this level on every level spans the same XP as the one before it
/// (3,080 XP). There is no level cap.
pub const LEVEL_FLAT_FROM: u32 = 26;

// --- Titles --------------------------------------------------------------

/// Title tier: from `min_level` on (until the next tier) the titles apply.
#[derive(Debug, Clone, Copy)]
pub struct TitleTier {
    pub min_level: u32,
    pub max: &'static str,
    pub diy: &'static str,
}

/// Ascending by `min_level`; the first tier starts at level 1. The Max title
/// also applies before onboarding.
pub const TITLE_TIERS: [TitleTier; 6] = [
    TitleTier {
        min_level: 1,
        max: "Token Tourist",
        diy: "Budget Curious",
    },
    TitleTier {
        min_level: 5,
        max: "Prompt Regular",
        diy: "Quiet Spender",
    },
    TitleTier {
        min_level: 10,
        max: "Context Hoarder",
        diy: "Frugal Prompter",
    },
    TitleTier {
        min_level: 15,
        max: "Cache Whisperer",
        diy: "Token Ascetic",
    },
    TitleTier {
        min_level: 20,
        max: "Limit Chaser",
        diy: "Zen Accountant",
    },
    TitleTier {
        min_level: 30,
        max: "Rate-Limit Royalty",
        diy: "Abbot of Restraint",
    },
];

// --- Achievements --------------------------------------------------------

/// Night Owl window ("coding after midnight"), local time, start inclusive,
/// end exclusive.
pub const NIGHT_START_HOUR: u32 = 0;
pub const NIGHT_END_HOUR: u32 = 4;

/// Rate-Limit Rambo: a 5-hour reading at or above this percentage.
pub const RAMBO_PERCENT: f64 = 100.0;
/// So Close: a finished 5-hour window whose peak is at least this and below
/// `RAMBO_PERCENT`.
pub const SO_CLOSE_PERCENT: f64 = 99.0;
/// Readings whose 5-hour reset times are within one bucket belong to one
/// window (the reset timestamp jitters by milliseconds between readings).
pub const WINDOW_BUCKET_MS: i64 = 60_000;

/// Token Monk: one finished day with a daily budget counts when its budget
/// usage is below this percentage.
pub const TOKEN_MONK_BELOW_PERCENT: f64 = 100.0;
/// Calendar days the caller supplies per evaluation (live), so a finished day
/// is still seen when an evaluation was missed.
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
        description: "Your first token, spent.",
        secret: false,
        mode: None,
        xp: 10,
        target: None,
    },
    AchievementDef {
        id: AchievementId::NightOwl,
        name: "Night Owl",
        description: "Coding after midnight.",
        secret: false,
        mode: Some(Mode::Max),
        xp: 50,
        target: None,
    },
    AchievementDef {
        id: AchievementId::RateLimitRambo,
        name: "Rate-Limit Rambo",
        description: "Hit the limit. On purpose, we assume.",
        secret: false,
        mode: Some(Mode::Max),
        xp: 100,
        target: None,
    },
    AchievementDef {
        id: AchievementId::SoClose,
        name: "So Close",
        description: "Stopped at 99 percent. Restraint or luck.",
        secret: false,
        mode: Some(Mode::Max),
        xp: 60,
        target: None,
    },
    AchievementDef {
        id: AchievementId::TokenMonk,
        name: "Token Monk",
        description: "A full day under budget.",
        secret: false,
        mode: Some(Mode::Diy),
        xp: 150,
        target: None,
    },
    AchievementDef {
        id: AchievementId::Boo,
        name: "Halloween 2026",
        description: "Found the ghost. It has since left the building.",
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
