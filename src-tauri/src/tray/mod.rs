//! Tray icon with the live halo gauge, and its menu.
//!
//! [`init`] creates the tray once; [`set_halo`] re-renders the icon. A left
//! click toggles the popup (see `crate::window`); the menu opens on right
//! click. Rendering lives in [`icon`] and is pure apart from the final
//! hand-over to the OS.

pub mod icon;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter};

use crate::contracts::events::{NavigateTo, View, NAVIGATE};
use crate::contracts::Mode;
use crate::window;

use icon::{render_icon, IconStyle, ICON_SIZE};

/// Id of the single tray icon.
pub const TRAY_ID: &str = "main";

const MENU_OPEN: &str = "open";
const MENU_SETTINGS: &str = "settings";
const MENU_QUIT: &str = "quit";

/// What a tray menu entry asks the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    Open,
    Settings,
    Quit,
}

impl MenuAction {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            MENU_OPEN => Some(Self::Open),
            MENU_SETTINGS => Some(Self::Settings),
            MENU_QUIT => Some(Self::Quit),
            _ => None,
        }
    }
}

/// Creates the tray icon (dotted track only, until the first [`set_halo`]) and its
/// menu. Call once from the setup hook, after the positioner plugin is
/// registered.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let style = IconStyle::for_current_platform();

    let open = MenuItem::with_id(app, MENU_OPEN, "Open Tokenmonk", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, MENU_SETTINGS, "Settings", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &separator, &quit])?;

    let image = build_image(style, None, Mode::NoneYet)
        .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(image)
        .icon_as_template(style.template)
        .tooltip("Tokenmonk")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(
            |app, event| match MenuAction::from_id(event.id().as_ref()) {
                Some(MenuAction::Open) => window::show(app),
                Some(MenuAction::Settings) => {
                    window::show(app);
                    if let Err(e) = app.emit(
                        NAVIGATE,
                        NavigateTo {
                            view: View::Settings,
                        },
                    ) {
                        log::warn!("could not emit navigate: {e}");
                    }
                }
                Some(MenuAction::Quit) => app.exit(0),
                None => {}
            },
        )
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            // The positioner needs every event to learn where the icon is.
            tauri_plugin_positioner::on_tray_event(app, &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::toggle(app);
            }
        })
        .build(app)?;
    Ok(())
}

/// Re-renders the tray icon with the arc at `percent` left (0 to 100,
/// clockwise from 12 o'clock) in the accent of `mode`. `None` shows the
/// dotted track only, for the "no limit data" state.
///
/// Failures are logged and leave the previous icon in place.
pub fn set_halo(app: &AppHandle, percent: Option<f64>, mode: Mode) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Err(e) = apply(&tray, percent, mode) {
        log::warn!("could not update the tray icon: {e}");
    }
}

fn apply(tray: &TrayIcon, percent: Option<f64>, mode: Mode) -> Result<(), String> {
    let style = IconStyle::for_current_platform();
    let image = build_image(style, percent, mode)?;
    // One call so macOS does not flicker between icon and template flag.
    tray.set_icon_with_as_template(Some(image), style.template)
        .map_err(|e| e.to_string())
}

fn build_image(
    style: IconStyle,
    percent: Option<f64>,
    mode: Mode,
) -> Result<Image<'static>, String> {
    let rgba = render_icon(style, percent, mode)?;
    Ok(Image::new_owned(rgba, ICON_SIZE, ICON_SIZE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_map_to_actions() {
        assert_eq!(MenuAction::from_id("open"), Some(MenuAction::Open));
        assert_eq!(MenuAction::from_id("settings"), Some(MenuAction::Settings));
        assert_eq!(MenuAction::from_id("quit"), Some(MenuAction::Quit));
        assert_eq!(MenuAction::from_id("other"), None);
    }
}
