//! Tokenmonk application core.

pub mod aggregate;
pub mod app;
pub mod commands;
pub mod contracts;
pub mod db;
pub mod game;
pub mod limits;
pub mod logs;
pub mod tray;
pub mod watcher;
pub mod window;

#[cfg(desktop)]
use tauri_plugin_autostart::MacosLauncher;

/// Builds and runs the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Status-line bridge mode: no window, tray, plugin or database starts
    // (docs/ARCHITECTURE.md, "Bridge mode").
    if std::env::args().nth(1).as_deref() == Some(limits::setup::BRIDGE_FLAG) {
        limits::bridge::run();
        return;
    }

    let mut builder = tauri::Builder::default();

    // Must be registered first so a second launch exits before anything else starts.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show(app);
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard,
            commands::get_settings,
            commands::update_settings,
            commands::complete_onboarding,
            commands::get_game_state,
            commands::get_app_info,
            commands::get_limit_setup,
            commands::get_update_status,
            commands::check_for_updates,
            commands::install_update,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                let handle = app.handle();
                handle.plugin(tauri_plugin_updater::Builder::new().build())?;
                handle.plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    None,
                ))?;
                handle.plugin(tauri_plugin_positioner::init())?;
            }
            app::setup(app)
        })
        .run(tauri::generate_context!())
        .expect("error while running Tokenmonk");
}
