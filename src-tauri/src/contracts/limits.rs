//! Subscription limits (5-hour and weekly) and the swappable source trait.
//!
//! Limit percentages include claude.ai chats; token statistics come from
//! Claude Code logs only. No concrete source is defined here: the `limits`
//! module provides implementations of [`LimitSource`]. v0.1 reads the file
//! written by the status-line bridge (docs/ARCHITECTURE.md, "Limits").
//! Without a working source the app runs on token data alone and says so
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

/// Returned by `get_limit_setup`: what the Settings view needs to show the
/// status-line setup snippet. Tokenmonk never edits Claude Code settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitSetup {
    /// Ready-to-paste value for `statusLine.command` in Claude Code's
    /// `settings.json`: this executable's path plus the bridge flag, for
    /// example `"/Applications/Tokenmonk.app/Contents/MacOS/tokenmonk --statusline"`.
    /// `None` if the executable path cannot be determined.
    pub statusline_command: Option<String>,
    /// State of the bridge file `statusline.json`, read when the command
    /// runs. Lets Settings tell "never arrived" from "arrived, no current
    /// window". Defaults to "missing" when absent on input (added after
    /// v0.1's first `LimitSetup`; output always carries it).
    #[serde(default)]
    pub statusline_file: StatuslineFileInfo,
}

/// What the app knows about the bridge file, without judging it. Part of
/// [`LimitSetup`]; see docs/ARCHITECTURE.md, "Setup snippet".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatuslineFileInfo {
    /// The file exists. `false` also when the app data directory is unknown
    /// (then the bridge cannot write it either).
    pub exists: bool,
    /// `receivedAt` from the file: when the bridge last ran. `None` if the
    /// file is missing, unreadable or not a valid bridge file.
    pub received_at: Option<EpochMs>,
    /// File modification time. Only adds value when `received_at` is `None`
    /// but the file exists (something arrived, but it cannot be read).
    /// `None` if the file is missing or the OS does not report it.
    pub modified_at: Option<EpochMs>,
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
