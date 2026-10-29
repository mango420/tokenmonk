//! The popup's data: token totals, session, burn rate, limits, daily budget.
//!
//! Definitions (see docs/ARCHITECTURE.md, "Aggregates"):
//! * "Today" is the current local calendar day.
//! * "This session" is the Claude Code session (`sessionId` in the logs,
//!   subagent sidechains included) with the most recent assistant message,
//!   as long as that message is younger than the idle cut-off.
//! * Headline token figures (`TokenBreakdown::total`, burn rate) count
//!   input + output + cache creation. Cache reads are reported separately
//!   and are not part of the headline.

use serde::{Deserialize, Serialize};

use super::EpochMs;
use super::limits::LimitsView;

/// Token counts for a period, deduplicated per API message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBreakdown {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
    /// Headline figure: `input + output + cache_creation`.
    pub total: u64,
}

/// The current Claude Code session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    /// `sessionId` from the logs.
    pub session_id: String,
    /// Last path component of the session's working directory.
    pub project_name: String,
    /// First assistant message of the session.
    pub started_at: EpochMs,
    /// Latest assistant message of the session.
    pub last_activity_at: EpochMs,
    pub tokens: TokenBreakdown,
}

/// Recent token burn and, if limits are known, the projected time to the
/// 5-hour limit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnRate {
    /// Headline tokens per minute over the trailing window.
    pub tokens_per_minute: f64,
    /// Length of the trailing window in minutes.
    pub window_minutes: u32,
    /// Minutes until the 5-hour limit is reached at the current pace.
    /// `None` when limits are unavailable or nothing is burning.
    /// `Some(0)` when the limit is already reached.
    pub limit_eta_minutes: Option<u32>,
}

/// What the daily budget is measured in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BudgetBasis {
    /// `"weeklyLimit"`: percentage points of the weekly limit.
    WeeklyLimit,
    /// `"tokenHistory"`: headline tokens, derived from past days because no
    /// limit data is available.
    TokenHistory,
}

/// How much may be used per day until the weekly reset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyBudget {
    pub basis: BudgetBasis,
    /// Today's allowance in the unit of `basis`.
    pub allowance: f64,
    /// Used today in the unit of `basis`.
    pub used_today: f64,
    /// `used_today / allowance * 100`; may exceed 100 (over budget).
    pub used_percent: f64,
    /// Days until the weekly reset, today included (at least 1).
    pub days_left: u32,
    /// Weekly reset, if known.
    pub resets_at: Option<EpochMs>,
}

/// State of the log import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataState {
    /// `"importing"`: first scan or catch-up in progress.
    Importing,
    /// `"ready"`: live.
    Ready,
    /// `"noLogs"`: the Claude Code projects folder is missing or empty.
    NoLogs,
    /// `"error"`: the log folder or the database cannot be read; see `detail`.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStatus {
    pub state: DataState,
    pub files_total: u32,
    pub files_done: u32,
    /// Timestamp of the newest assistant message seen in any log.
    pub last_message_at: Option<EpochMs>,
    pub detail: Option<String>,
}

/// Everything the popup needs. Returned by `get_dashboard` and pushed with
/// the `dashboard-updated` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub generated_at: EpochMs,
    pub today: TokenBreakdown,
    pub session: Option<SessionStats>,
    pub burn_rate: BurnRate,
    pub limits: LimitsView,
    /// `None` when neither limits nor enough token history exist.
    pub daily_budget: Option<DailyBudget>,
    pub data: DataStatus,
}
