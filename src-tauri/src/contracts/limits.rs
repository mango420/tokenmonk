//! Subscription limits (5-hour and weekly) and the swappable source trait.
//!
//! Limit percentages include claude.ai chats; token statistics come from
//! Claude Code logs only. No concrete source is defined here: the `limits`
//! module provides implementations of [`LimitSource`]. v0.1 reads the file
//! written by the status-line bridge (docs/ARCHITECTURE.md, "Limits").
//! Without a working source the app runs on token data alone and says so
//! ([`LimitStatus::Unavailable`]).
//!
//! [`LimitsConnection`] is the one-click setup (TKM-287): whether Claude
//! Code's `statusLine` runs the bridge (docs/ARCHITECTURE.md, "Connect
//! limits").

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
    /// `"noSource"`: no limit source is set up. In v0.1: the status-line
    /// bridge has not written a reading yet, or no window in it is current.
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

/// Whether Claude Code's status line runs the Tokenmonk bridge, read from
/// `settings.json` in the effective Claude folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LimitsConnectionState {
    /// `"notConnected"`: no Tokenmonk entry in `statusLine` (or no
    /// `settings.json` yet). Settings offers "Connect limits".
    NotConnected,
    /// `"connected"`: `statusLine` runs the bridge, alone or wrapping the
    /// previous status line, in the call form of the shell detected now.
    /// Settings offers "Disconnect".
    Connected,
    /// `"outdated"`: `statusLine` is Tokenmonk's, but its call form does not
    /// match the shell detected now (PowerShell form while Git Bash is
    /// found, or POSIX form on Windows without Git Bash). Nothing is
    /// rewritten on its own; Settings offers "Reconnect" (`connect_limits`).
    Outdated,
    /// `"blocked"`: the file cannot be used, see `problem`. Connect and
    /// disconnect would fail; nothing is touched.
    Blocked,
}

/// Why a [`LimitsConnection`] is `blocked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LimitsConnectionProblem {
    /// `"noClaudeFolder"`: the effective Claude folder is unknown or does
    /// not exist. Tokenmonk never creates it.
    NoClaudeFolder,
    /// `"settingsUnreadable"`: `settings.json` exists but cannot be read.
    SettingsUnreadable,
    /// `"settingsInvalid"`: `settings.json` is not a JSON object, or its
    /// `statusLine` is present but not an object. Tokenmonk never rewrites
    /// a file it cannot parse.
    SettingsInvalid,
}

/// Returned by `get_limits_connection`, `connect_limits` and
/// `disconnect_limits`, and pushed with `limits-connection-changed`.
/// Read fresh from `settings.json` on every call; nothing is cached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsConnection {
    pub state: LimitsConnectionState,
    /// Absolute path of `settings.json` in the effective Claude folder, for
    /// display. `None` only when the Claude folder is unknown.
    pub settings_path: Option<String>,
    /// A status line other than Tokenmonk's is set up. `connected` or
    /// `outdated`: it is wrapped and runs after the bridge with its output
    /// unchanged.
    /// `notConnected`: "Connect limits" will wrap it. Always `false` while
    /// `blocked`.
    pub previous_status_line: bool,
    /// Set exactly when `state` is `blocked`.
    pub problem: Option<LimitsConnectionProblem>,
    /// Short technical detail for logs, never shown as headline.
    pub detail: Option<String>,
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
