//! In-app update check and install (docs/ARCHITECTURE.md, "Updates").
//!
//! [`UpdateState`] holds the current [`UpdateStatus`] and, after an
//! `available` result, the plugin's `Update` for the install. Every
//! transition is stored and emitted as `update-status-changed`. The status
//! mapping is plain functions so it can be tested without network.

use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::contracts::events::UPDATE_STATUS_CHANGED;
use crate::contracts::update::release_page_url;
use crate::contracts::{CommandError, CommandErrorCode, EpochMs, UpdateStatus};

/// Network timeout of one check, in seconds.
#[cfg(desktop)]
const CHECK_TIMEOUT_SECS: u64 = 20;

/// Current status plus the pending plugin update.
#[derive(Default)]
pub struct UpdateState {
    status: Mutex<UpdateStatus>,
    #[cfg(desktop)]
    pending: Mutex<Option<tauri_plugin_updater::Update>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn emit(app: &AppHandle, status: &UpdateStatus) {
    if let Err(e) = app.emit(UPDATE_STATUS_CHANGED, status) {
        log::warn!("could not emit {UPDATE_STATUS_CHANGED}: {e}");
    }
}

impl UpdateState {
    pub fn status(&self) -> UpdateStatus {
        lock(&self.status).clone()
    }

    fn set(&self, app: &AppHandle, status: UpdateStatus) -> UpdateStatus {
        *lock(&self.status) = status.clone();
        emit(app, &status);
        status
    }

    /// Moves to `next` unless a check or install is already running; then
    /// the current status comes back as `Err`.
    fn begin(&self, app: &AppHandle, next: UpdateStatus) -> Result<UpdateStatus, UpdateStatus> {
        let mut guard = lock(&self.status);
        if guard.is_busy() {
            return Err(guard.clone());
        }
        *guard = next.clone();
        drop(guard);
        emit(app, &next);
        Ok(next)
    }
}

fn now_ms() -> EpochMs {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Whether the plugin config holds a non-blank `pubkey` for the updater.
pub(crate) fn has_pubkey<'a>(plugins: impl IntoIterator<Item = (&'a String, &'a Value)>) -> bool {
    plugins
        .into_iter()
        .find(|(name, _)| name.as_str() == "updater")
        .and_then(|(_, cfg)| cfg.get("pubkey"))
        .and_then(Value::as_str)
        .is_some_and(|key| !key.trim().is_empty())
}

pub(crate) fn status_available(version: &str, checked_at: EpochMs) -> UpdateStatus {
    let version = version.strip_prefix('v').unwrap_or(version).to_string();
    UpdateStatus::Available {
        url: release_page_url(&version),
        version,
        checked_at,
    }
}

pub(crate) fn status_check_failed(
    error: &dyn std::fmt::Display,
    checked_at: EpochMs,
) -> UpdateStatus {
    UpdateStatus::CheckFailed {
        checked_at,
        detail: Some(error.to_string()),
    }
}

#[cfg(desktop)]
pub(crate) fn status_install_failed(version: &str, error: &dyn std::fmt::Display) -> UpdateStatus {
    UpdateStatus::InstallFailed {
        version: version.to_string(),
        url: release_page_url(version),
        detail: Some(error.to_string()),
    }
}

fn nothing_to_install() -> CommandError {
    CommandError {
        code: CommandErrorCode::InvalidArgument,
        message: "no update is available to install".into(),
    }
}

/// Runs one check and returns the final status. Never panics.
#[cfg(desktop)]
pub async fn check(app: &AppHandle) -> UpdateStatus {
    use tauri_plugin_updater::UpdaterExt;

    let state = app.state::<UpdateState>();
    if state.status().is_busy() {
        return state.status();
    }
    if !has_pubkey(app.config().plugins.0.iter()) {
        return state.set(app, UpdateStatus::Unavailable);
    }
    let built = app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(CHECK_TIMEOUT_SECS))
        .build();
    let updater = match built {
        Ok(u) => u,
        Err(e) => {
            log::error!("could not build the updater: {e}");
            return state.set(app, UpdateStatus::Unavailable);
        }
    };
    if let Err(current) = state.begin(app, UpdateStatus::Checking) {
        return current;
    }
    match updater.check().await {
        Ok(Some(update)) => {
            let status = status_available(&update.version, now_ms());
            *lock(&state.pending) = Some(update);
            state.set(app, status)
        }
        Ok(None) => {
            *lock(&state.pending) = None;
            state.set(
                app,
                UpdateStatus::UpToDate {
                    checked_at: now_ms(),
                },
            )
        }
        Err(e) => {
            log::warn!("update check failed: {e}");
            state.set(app, status_check_failed(&e, now_ms()))
        }
    }
}

#[cfg(not(desktop))]
pub async fn check(app: &AppHandle) -> UpdateStatus {
    app.state::<UpdateState>()
        .set(app, UpdateStatus::Unavailable)
}

/// Downloads and installs the pending update. On success the app exits
/// (Windows) or restarts (macOS, Linux), so this normally does not return.
pub async fn install(app: &AppHandle) -> Result<UpdateStatus, CommandError> {
    let state = app.state::<UpdateState>();
    let current = state.status();
    if current.is_busy() {
        return Ok(current);
    }
    let version = current
        .installable_version()
        .ok_or_else(nothing_to_install)?
        .to_string();
    install_pending(app, &state, version).await
}

#[cfg(desktop)]
async fn install_pending(
    app: &AppHandle,
    state: &UpdateState,
    version: String,
) -> Result<UpdateStatus, CommandError> {
    let pending = lock(&state.pending).clone();
    let Some(update) = pending else {
        return Err(nothing_to_install());
    };
    let installing = UpdateStatus::Installing {
        version: version.clone(),
    };
    if let Err(current) = state.begin(app, installing) {
        return Ok(current);
    }
    match update.download_and_install(|_, _| {}, || {}).await {
        // Windows never gets here (the plugin exits); macOS and Linux need
        // a relaunch of the replaced bundle.
        Ok(()) => app.restart(),
        Err(e) => {
            log::warn!("update install failed: {e}");
            Ok(state.set(app, status_install_failed(&version, &e)))
        }
    }
}

#[cfg(not(desktop))]
async fn install_pending(
    _app: &AppHandle,
    _state: &UpdateState,
    _version: String,
) -> Result<UpdateStatus, CommandError> {
    Err(nothing_to_install())
}

/// Registers the state and, if the setting is on, spawns the one start check.
pub fn init(app: &AppHandle, check_at_start: bool) {
    app.manage(UpdateState::default());
    if check_at_start {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            check(&handle).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn plugins(v: Value) -> HashMap<String, Value> {
        HashMap::from([("updater".to_string(), v)])
    }

    #[test]
    fn empty_missing_or_blank_pubkey_is_unavailable() {
        assert!(!has_pubkey(plugins(json!({ "pubkey": "" })).iter()));
        assert!(!has_pubkey(plugins(json!({ "pubkey": "  \n" })).iter()));
        assert!(!has_pubkey(plugins(json!({ "endpoints": [] })).iter()));
        assert!(!has_pubkey(plugins(json!({ "pubkey": 5 })).iter()));
        assert!(!has_pubkey(HashMap::new().iter()));
        assert!(has_pubkey(
            plugins(json!({ "pubkey": "dW50cnVzdGVk" })).iter()
        ));
    }

    #[test]
    fn available_strips_v_and_links_the_release_page() {
        let s = status_available("v0.4.1", 7);
        assert_eq!(
            s,
            UpdateStatus::Available {
                version: "0.4.1".into(),
                url: "https://github.com/mango420/tokenmonk/releases/tag/v0.4.1".into(),
                checked_at: 7,
            }
        );
    }

    #[cfg(desktop)]
    #[test]
    fn errors_map_to_failed_statuses() {
        let e = tauri_plugin_updater::Error::ReleaseNotFound;
        match status_check_failed(&e, 9) {
            UpdateStatus::CheckFailed { checked_at, detail } => {
                assert_eq!(checked_at, 9);
                assert!(detail.is_some());
            }
            other => panic!("unexpected {other:?}"),
        }
        let s = status_install_failed("0.4.1", &e);
        assert_eq!(s.installable_version(), Some("0.4.1"));
        assert!(matches!(s, UpdateStatus::InstallFailed { .. }));
    }
}
