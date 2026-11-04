//! Shared contracts between the Rust core and the web UI.
//!
//! Everything in here is mirrored 1:1 in `src/lib/contracts.ts`. Change both
//! sides together, and only through the architect (see docs/ARCHITECTURE.md).
//!
//! Dependencies: `serde` (derive) and `serde_json` only. No Tauri types, so
//! the module compiles and tests without the app shell. Declare it as
//! `pub mod contracts;` in the crate root.
//!
//! # Serialization rules
//!
//! * Struct fields: `#[serde(rename_all = "camelCase")]`, so `resets_at`
//!   becomes `resetsAt` on the wire.
//! * Unit enums serialize as camelCase strings (`Mode::NoneYet` ->
//!   `"noneYet"`, `AchievementId::RateLimitRambo` -> `"rateLimitRambo"`).
//! * Data-carrying enums (only `UpdateStatus`) are internally tagged:
//!   `#[serde(tag = "kind", rename_all = "camelCase")]` on the enum names
//!   the variants, and every struct variant carries its own
//!   `#[serde(rename_all = "camelCase")]` for its fields
//!   (`UpdateStatus::UpToDate { checked_at }` ->
//!   `{ "kind": "upToDate", "checkedAt": .. }`). TypeScript mirrors them as
//!   a discriminated union on `kind`.
//! * `Option<T>` serializes as `null` (never omitted) and maps to `T | null`
//!   in TypeScript. Exception: patch types (`SettingsPatch`) where an absent
//!   field means "leave unchanged"; those map to optional `field?: T`.
//! * Timestamps are [`EpochMs`]: Unix epoch milliseconds, UTC, as `i64`.
//!   The UI formats them in local time.
//! * Token counts are `u64`; they stay far below 2^53 and are safe as JS
//!   numbers. Floats must never be NaN or infinite (serde_json would emit
//!   `null`); producers clamp or use `None` instead.
//! * Command names are snake_case (they equal the Rust function names);
//!   command arguments are single lowercase words so Tauri's camelCase
//!   argument mapping never changes them. Event names are kebab-case.

pub mod app;
pub mod commands;
pub mod dashboard;
pub mod events;
pub mod game;
pub mod limits;
pub mod schema;
pub mod settings;
pub mod update;

pub use app::{AppInfo, Platform};
pub use commands::{CommandError, CommandErrorCode, CompleteOnboardingArgs, UpdateSettingsArgs};
pub use dashboard::{
    BudgetBasis, BurnRate, DailyBudget, DashboardSnapshot, DataState, DataStatus, SessionStats,
    TokenBreakdown,
};
pub use events::{NavigateTo, View};
pub use game::{AchievementId, AchievementProgress, AchievementUnlocked, GameState, LevelUp};
pub use limits::{
    LimitError, LimitReading, LimitReason, LimitSetup, LimitSource, LimitStatus, LimitWindow,
    LimitsView,
};
pub use settings::{Mode, Settings, SettingsPatch};
pub use update::UpdateStatus;

/// Unix epoch milliseconds, UTC.
pub type EpochMs = i64;

#[cfg(test)]
mod tests;
