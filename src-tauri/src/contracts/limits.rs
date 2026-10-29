//! Subscription limits (5-hour and weekly) and the swappable source trait.
//!
//! Limit percentages include claude.ai chats; token statistics come from
//! Claude Code logs only. No concrete source is defined here: the `limits`
//! module provides implementations of [`LimitSource`]. Without a working
//! source the app runs on token data alone and says so
//! ([`LimitStatus::Unavailable`]).

use serde::{Deserialize, Serialize};
use std::fmt;

use super::EpochMs;

/// One limit window as reported by a source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitWindow {
    /// Used share of the window in percent, `0.0..=100.0` (may exceed 100
    /// if a source reports overage; the UI clamps the gauge, not the label).
    pub utilization: f64,
    /// When the window resets, if the source knows it.
    pub resets_at: Option<EpochMs>,
}

/// A raw reading from a [`LimitSource`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitReading {
    pub five_hour: Option<LimitWindow>,
    pub weekly: Option<LimitWindow>,
    /// When the source produced the values.
    pub fetched_at: EpochMs,
}

/// Health of the limit data shown in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LimitStatus {
    /// `"ok"`: a recent reading exists.
    Ok,
    /// `"stale"`: the last good reading is older than the stale threshold
    /// (see docs/ARCHITECTURE.md); values are still shown, marked as old.
    Stale,
    /// `"unavailable"`: no usable reading; the app runs on token data only.
    Unavailable,
}

/// Why limit data is stale or unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LimitReason {
    /// `"noSource"`: no limit source is configured in this build.
    NoSource,
    /// `"notSignedIn"`: the source needs credentials that are missing or expired.
    NotSignedIn,
    /// `"network"`: the source could not be reached.
    Network,
    /// `"invalidResponse"`: the source answered with something unexpected.
    InvalidResponse,
    /// `"other"`: anything else; see `detail`.
    Other,
}

/// Limit data as the UI sees it (part of `DashboardSnapshot`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsView {
    pub status: LimitStatus,
    /// `None` when `status` is `ok`.
    pub reason: Option<LimitReason>,
    /// Short technical detail for logs and tooltips, never shown as headline.
    pub detail: Option<String>,
    /// Identifier of the source that produced the values (see [`LimitSource::id`]).
    pub source: Option<String>,
    pub five_hour: Option<LimitWindow>,
    pub weekly: Option<LimitWindow>,
    /// Time of the last good reading.
    pub fetched_at: Option<EpochMs>,
}

/// Error returned by a [`LimitSource`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitError {
    pub reason: LimitReason,
    pub detail: String,
}

impl fmt::Display for LimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "limit source error ({:?}): {}", self.reason, self.detail)
    }
}

impl std::error::Error for LimitError {}

/// A swappable provider of limit data.
///
/// Implementations live in the `limits` module. `fetch` is blocking and is
/// called from the limits worker thread, never from the UI thread or a
/// command handler. Implementations must not panic; every failure is a
/// [`LimitError`].
pub trait LimitSource: Send + Sync {
    /// Stable, short identifier (for example `"none"`), shown in
    /// `LimitsView::source`.
    fn id(&self) -> &'static str;

    /// Fetch the current utilisation of both windows.
    fn fetch(&self) -> Result<LimitReading, LimitError>;
}
