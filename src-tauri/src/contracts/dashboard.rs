//! The popup's data: token totals, session, burn rate and limits. No
//! estimates: no budget, no forecast, no ETA (docs/DESIGN.md, "Product
//! line").
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

use super::limits::LimitsView;
use super::EpochMs;

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

/// Recent token burn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnRate {
    /// Headline tokens per minute over the trailing window.
    pub tokens_per_minute: f64,
    /// Length of the trailing window in minutes.
    pub window_minutes: u32,
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
    pub data: DataStatus,
}
