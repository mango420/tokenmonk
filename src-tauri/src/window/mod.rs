//! Popup window: placement next to the tray icon, show, hide and toggle.
//!
//! The popup is the single `main` window declared in `tauri.conf.json`
//! (hidden at start, no decorations). It hides when it loses focus and
//! when the user closes it; the app keeps living in the tray. The header is a
//! drag region; once the user drags the popup away from the tray it opens
//! where they left it until the app restarts. The frontend may also close or
//! toggle-maximize it; a maximized popup is restored before it hides, so the
//! remembered position never describes the maximized frame.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::limits::LimitsHandle;

/// Label of the popup window in `tauri.conf.json`.
pub const LABEL: &str = "main";

/// A tray click this soon after the popup hid itself on blur is the click
/// that moved focus away, so it must not reopen the popup.
const BLUR_GRACE: Duration = Duration::from_millis(250);

static LAST_BLUR_HIDE: Mutex<Option<Instant>> = Mutex::new(None);

/// Blur is judged only after this delay, so a focus flicker that comes with a
/// native move loop can settle first.
const BLUR_SETTLE: Duration = Duration::from_millis(100);

/// A window move this recent means the blur belongs to a drag, not to the user
/// clicking elsewhere.
const DRAG_WINDOW: Duration = Duration::from_millis(400);

/// When the popup last reported a move (drag, maximize or `place`).
static LAST_MOVE: Mutex<Option<Instant>> = Mutex::new(None);

/// Moves the app causes itself (`place`, maximize, restore) report `Moved`
/// too. Moves and resizes reported until this instant are not drags.
static APP_MOVE_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// How long after an app-initiated move or resize `Moved` events are ignored.
const APP_MOVE_SLACK: Duration = Duration::from_millis(300);

fn mark_app_move() {
    if let Ok(mut until) = APP_MOVE_UNTIL.lock() {
        *until = Some(Instant::now() + APP_MOVE_SLACK);
    }
}

fn in_app_move() -> bool {
    APP_MOVE_UNTIL
        .lock()
        .ok()
        .and_then(|until| *until)
        .is_some_and(|t| Instant::now() < t)
}

/// Where `place` last put the popup. If it sits elsewhere when it opens
/// again, the user dragged it there.
static LAST_PLACED: Mutex<Option<PhysicalPosition<i32>>> = Mutex::new(None);

/// Rounding slack between the position `place` asked for and the one the OS
/// reports back, in physical pixels.
const MOVE_TOLERANCE: i32 = 2;

fn moved_by_user(
    placed: Option<PhysicalPosition<i32>>,
    current: Option<PhysicalPosition<i32>>,
) -> bool {
    match (placed, current) {
        (Some(p), Some(c)) => {
            (p.x - c.x).abs() > MOVE_TOLERANCE || (p.y - c.y).abs() > MOVE_TOLERANCE
        }
        _ => false,
    }
}

/// True if the centre of a window at `pos` with `size` lies on one of the
/// `screens` (position and size of each monitor).
fn centre_on_screen(
    pos: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    screens: &[(PhysicalPosition<i32>, PhysicalSize<u32>)],
) -> bool {
    let cx = i64::from(pos.x) + i64::from(size.width) / 2;
    let cy = i64::from(pos.y) + i64::from(size.height) / 2;
    screens.iter().any(|(p, s)| {
        let (x, y) = (i64::from(p.x), i64::from(p.y));
        (x..x + i64::from(s.width)).contains(&cx) && (y..y + i64::from(s.height)).contains(&cy)
    })
}

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

/// What to do once a blur has settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlurAction {
    Hide,
    /// The blur came with a drag: take the focus back instead of hiding.
    Refocus,
    Ignore,
}

fn blur_action(visible: bool, focused: bool, since_move: Option<Duration>) -> BlurAction {
    if !visible || focused {
        BlurAction::Ignore
    } else if since_move.is_some_and(|d| d < DRAG_WINDOW) {
        BlurAction::Refocus
    } else {
        BlurAction::Hide
    }
}

/// Unmaximizes the popup if needed, without counting it as a drag.
fn restore(popup: &WebviewWindow) {
    if popup.is_maximized().unwrap_or(false) {
        mark_app_move();
        let _ = popup.unmaximize();
    }
}

/// Judges a blur until it is settled: hides the popup, or takes the focus back
/// while a drag is going on. Focus can be refused, so it asks again after each
/// refocus; that ends because a refocus emits no `Moved` and `DRAG_WINDOW`
/// runs out.
fn settle_blur(handle: &WebviewWindow) {
    loop {
        std::thread::sleep(BLUR_SETTLE);
        let since_move = LAST_MOVE
            .lock()
            .ok()
            .and_then(|last| last.map(|t| t.elapsed()));
        let visible = handle.is_visible().unwrap_or(false);
        let focused = handle.is_focused().unwrap_or(false);
        match blur_action(visible, focused, since_move) {
            BlurAction::Hide => {
                if let Ok(mut last) = LAST_BLUR_HIDE.lock() {
                    *last = Some(Instant::now());
                }
                dismiss(handle);
                return;
            }
            BlurAction::Refocus => {
                let _ = handle.set_focus();
            }
            BlurAction::Ignore => return,
        }
    }
}

/// Restores a maximized popup, then hides it. Restoring happens while the
/// window is still visible, because a hidden window ignores it on Windows.
fn dismiss(popup: &WebviewWindow) {
    restore(popup);
    let _ = popup.hide();
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
            let handle = handle.clone();
            std::thread::spawn(move || settle_blur(&handle));
        }
        WindowEvent::Moved(_) => {
            let maximized = handle.is_maximized().unwrap_or(false);
            if !maximized && !in_app_move() {
                if let Ok(mut last) = LAST_MOVE.lock() {
                    *last = Some(Instant::now());
                }
            }
        }
        // A size change (maximize or restore, from the app or the frontend)
        // brings a move that is not a drag.
        WindowEvent::Resized(_) => mark_app_move(),
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            dismiss(&handle);
        }
        _ => {}
    });
    Ok(())
}

/// Shows the popup next to the tray icon (or where the user dragged it) and
/// focuses it. Asks the limits
/// worker for a fresh reading (it skips the request if the last poll is
/// recent). Also the right reaction to a second app launch.
pub fn show(app: &AppHandle) {
    let Some(popup) = app.get_webview_window(LABEL) else {
        return;
    };
    restore(&popup);
    mark_app_move();
    if !stays_where_dragged(&popup) {
        place(&popup);
    }
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
        dismiss(&popup);
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

/// The user dragged the popup away from where `place` put it, and it is still
/// on a connected monitor (the one it was dragged to may be unplugged since).
fn stays_where_dragged(popup: &WebviewWindow) -> bool {
    let placed = LAST_PLACED.lock().ok().and_then(|last| *last);
    let (Ok(pos), Ok(size), Ok(monitors)) = (
        popup.outer_position(),
        popup.outer_size(),
        popup.available_monitors(),
    ) else {
        return false;
    };
    let screens: Vec<_> = monitors
        .iter()
        .map(|m| (*m.position(), *m.size()))
        .collect();
    moved_by_user(placed, Some(pos)) && centre_on_screen(pos, size, &screens)
}

/// Moves the popup next to the tray icon, kept inside the screen. Before the
/// tray position is known (no tray event yet) it falls back to the screen
/// corner where the tray usually sits.
fn place(popup: &WebviewWindow) {
    if popup.move_window_constrained(Position::TrayCenter).is_err() {
        let corner = if cfg!(target_os = "macos") {
            Position::TopRight
        } else {
            Position::BottomRight
        };
        if let Err(e) = popup.move_window(corner) {
            log::warn!("could not position the popup: {e}");
            return;
        }
    }
    if let Ok(mut last) = LAST_PLACED.lock() {
        *last = popup.outer_position().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_without_recent_move_hides() {
        assert_eq!(blur_action(true, false, None), BlurAction::Hide);
        let old = Some(DRAG_WINDOW + Duration::from_millis(1));
        assert_eq!(blur_action(true, false, old), BlurAction::Hide);
    }

    #[test]
    fn blur_during_a_drag_refocuses() {
        let recent = Some(Duration::from_millis(20));
        assert_eq!(blur_action(true, false, recent), BlurAction::Refocus);
    }

    #[test]
    fn refused_refocus_ends_in_hide() {
        // Refocus fails every time, so only the time since the last move grows.
        let mut since_move = Duration::from_millis(20);
        let mut rounds = 0;
        let last = loop {
            let action = blur_action(true, false, Some(since_move));
            if action != BlurAction::Refocus {
                break action;
            }
            rounds += 1;
            since_move += BLUR_SETTLE;
        };
        assert_eq!(last, BlurAction::Hide);
        assert!(rounds >= 1);
    }

    #[test]
    fn settled_or_hidden_blur_is_ignored() {
        assert_eq!(blur_action(true, true, None), BlurAction::Ignore);
        assert_eq!(blur_action(false, false, None), BlurAction::Ignore);
    }

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
    fn dragged_popup_stays_put() {
        let at = |x, y| Some(PhysicalPosition::new(x, y));
        assert!(!moved_by_user(None, at(10, 10)));
        assert!(!moved_by_user(at(10, 10), None));
        assert!(!moved_by_user(at(10, 10), at(11, 9)));
        assert!(moved_by_user(at(10, 10), at(10, 200)));
    }

    #[test]
    fn popup_on_unplugged_monitor_is_off_screen() {
        let popup = PhysicalSize::new(360, 680);
        let main = (PhysicalPosition::new(0, 0), PhysicalSize::new(1920, 1080));
        let left = (
            PhysicalPosition::new(-1280, 0),
            PhysicalSize::new(1280, 1024),
        );
        let on_left = PhysicalPosition::new(-800, 100);
        assert!(centre_on_screen(on_left, popup, &[main, left]));
        assert!(!centre_on_screen(on_left, popup, &[main]));
        assert!(centre_on_screen(
            PhysicalPosition::new(1700, 300),
            popup,
            &[main]
        ));
        assert!(!centre_on_screen(
            PhysicalPosition::new(1800, 300),
            popup,
            &[main]
        ));
    }

    #[test]
    fn click_right_after_blur_does_not_reopen() {
        assert_eq!(
            toggle_action(false, Some(Duration::from_millis(40))),
            ToggleAction::Nothing
        );
    }
}
