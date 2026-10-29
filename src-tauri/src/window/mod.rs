//! Popup window: placement next to the tray icon, show, hide and toggle.
//!
//! The popup is the single `main` window declared in `tauri.conf.json`
//! (hidden at start, no decorations). It hides when it loses focus and
//! when the user closes it; the app keeps living in the tray.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, WebviewWindow, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::limits::LimitsHandle;

/// Label of the popup window in `tauri.conf.json`.
pub const LABEL: &str = "main";

/// A tray click this soon after the popup hid itself on blur is the click
/// that moved focus away, so it must not reopen the popup.
const BLUR_GRACE: Duration = Duration::from_millis(250);

static LAST_BLUR_HIDE: Mutex<Option<Instant>> = Mutex::new(None);

/// Outcome of a tray click on a popup in a given state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToggleAction {
    Show,
    Hide,
    /// The click only dismissed the popup via blur; do nothing more.
    Nothing,
}

fn toggle_action(visible: bool, since_blur_hide: Option<Duration>) -> ToggleAction {
    if visible {
        ToggleAction::Hide
    } else if since_blur_hide.is_some_and(|d| d < BLUR_GRACE) {
        ToggleAction::Nothing
    } else {
        ToggleAction::Show
    }
}

/// Prepares the popup: hides it on blur and on close, and on macOS keeps the
/// app out of the Dock. Call once from the setup hook.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory)?;

    let Some(popup) = app.get_webview_window(LABEL) else {
        return Err(tauri::Error::WindowNotFound);
    };
    let handle = popup.clone();
    popup.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => {
            if handle.is_visible().unwrap_or(false) {
                if let Ok(mut last) = LAST_BLUR_HIDE.lock() {
                    *last = Some(Instant::now());
                }
                let _ = handle.hide();
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = handle.hide();
        }
        _ => {}
    });
    Ok(())
}

/// Shows the popup next to the tray icon and focuses it. Asks the limits
/// worker for a fresh reading (it skips the request if the last poll is
/// recent). Also the right reaction to a second app launch.
pub fn show(app: &AppHandle) {
    let Some(popup) = app.get_webview_window(LABEL) else {
        return;
    };
    place(&popup);
    if let Err(e) = popup.show() {
        log::warn!("could not show the popup: {e}");
        return;
    }
    let _ = popup.set_focus();
    if let Some(limits) = app.try_state::<LimitsHandle>() {
        limits.refresh();
    }
}

/// Hides the popup.
pub fn hide(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window(LABEL) {
        let _ = popup.hide();
    }
}

/// Tray click: shows the popup if hidden, hides it if visible.
pub fn toggle(app: &AppHandle) {
    let Some(popup) = app.get_webview_window(LABEL) else {
        return;
    };
    let visible = popup.is_visible().unwrap_or(false);
    let since_blur = LAST_BLUR_HIDE
        .lock()
        .ok()
        .and_then(|last| last.map(|t| t.elapsed()));
    match toggle_action(visible, since_blur) {
        ToggleAction::Show => show(app),
        ToggleAction::Hide => hide(app),
        ToggleAction::Nothing => {}
    }
}

/// Moves the popup next to the tray icon, kept inside the screen. Before the
/// tray position is known (no tray event yet) it falls back to the screen
/// corner where the tray usually sits.
fn place(popup: &WebviewWindow) {
    if popup.move_window_constrained(Position::TrayCenter).is_ok() {
        return;
    }
    let corner = if cfg!(target_os = "macos") {
        Position::TopRight
    } else {
        Position::BottomRight
    };
    if let Err(e) = popup.move_window(corner) {
        log::warn!("could not position the popup: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_popup_hides() {
        assert_eq!(toggle_action(true, None), ToggleAction::Hide);
    }

    #[test]
    fn hidden_popup_shows() {
        assert_eq!(toggle_action(false, None), ToggleAction::Show);
        assert_eq!(
            toggle_action(false, Some(Duration::from_secs(5))),
            ToggleAction::Show
        );
    }

    #[test]
    fn click_right_after_blur_does_not_reopen() {
        assert_eq!(
            toggle_action(false, Some(Duration::from_millis(40))),
            ToggleAction::Nothing
        );
    }
}
