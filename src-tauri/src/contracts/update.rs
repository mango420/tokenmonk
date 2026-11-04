//! In-app update check (TKM-41). Rust runs the check and the install
//! through `tauri-plugin-updater`; the UI only reads [`UpdateStatus`] with
//! `get_update_status`, follows `update-status-changed` and calls
//! `check_for_updates` / `install_update`. See docs/ARCHITECTURE.md
//! ("Updates").

use serde::{Deserialize, Serialize};

use super::EpochMs;

/// Base of the GitHub release page of one version. The page of `0.4.1` is
/// `RELEASE_PAGE_BASE` + `"v0.4.1"` (tags are `v<version>`, see
/// docs/RELEASE-FLOW.md).
pub const RELEASE_PAGE_BASE: &str = "https://github.com/mango420/tokenmonk/releases/tag/";

/// State of the update check. Data-carrying, so internally tagged with
/// `"kind"`; variant names and field names are camelCase on the wire.
///
/// Returned by `get_update_status`, `check_for_updates` and `install_update`
/// and pushed with `update-status-changed` on every transition.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateStatus {
    /// `{ "kind": "idle" }`: no check has run since start (automatic check
    /// off, or the start check has not begun yet).
    #[default]
    Idle,
    /// `{ "kind": "checking" }`: a check is running.
    Checking,
    /// `{ "kind": "upToDate", "checkedAt": .. }`: the running version is the
    /// newest one.
    #[serde(rename_all = "camelCase")]
    UpToDate { checked_at: EpochMs },
    /// `{ "kind": "available", "version", "url", "checkedAt" }`: a newer
    /// version exists and can be installed with `install_update`.
    #[serde(rename_all = "camelCase")]
    Available {
        /// New version without a leading "v", for example "0.4.1".
        version: String,
        /// GitHub release page of that version ([`RELEASE_PAGE_BASE`]).
        url: String,
        checked_at: EpochMs,
    },
    /// `{ "kind": "unavailable" }`: this build cannot update itself (empty
    /// updater public key, as in development and builds without updater
    /// artifacts, or the updater could not be built). No request is made.
    Unavailable,
    /// `{ "kind": "checkFailed", "checkedAt", "detail" }`: offline, network
    /// error, timeout, 404 or invalid `latest.json`. The UI shows a calm
    /// "Couldn't check"; no popup.
    #[serde(rename_all = "camelCase")]
    CheckFailed {
        checked_at: EpochMs,
        /// Technical message for logs; the UI shows its own copy.
        detail: Option<String>,
    },
    /// `{ "kind": "installing", "version" }`: download and install running.
    /// On success the app exits (Windows installer) or restarts (macOS), so
    /// no further status follows.
    #[serde(rename_all = "camelCase")]
    Installing { version: String },
    /// `{ "kind": "installFailed", "version", "url", "detail" }`: download,
    /// signature check or install failed. `install_update` may be retried.
    #[serde(rename_all = "camelCase")]
    InstallFailed {
        version: String,
        url: String,
        /// Technical message for logs; the UI shows its own copy.
        detail: Option<String>,
    },
}

impl UpdateStatus {
    /// Whether a check or an install is running; `check_for_updates` and
    /// `install_update` then return the current status without starting
    /// anything.
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Checking | Self::Installing { .. })
    }

    /// The version `install_update` would install, if any.
    pub fn installable_version(&self) -> Option<&str> {
        match self {
            Self::Available { version, .. } | Self::InstallFailed { version, .. } => Some(version),
            _ => None,
        }
    }
}

/// GitHub release page of `version` (a leading "v" is ignored).
pub fn release_page_url(version: &str) -> String {
    let bare = version.strip_prefix('v').unwrap_or(version);
    format!("{RELEASE_PAGE_BASE}v{bare}")
}
