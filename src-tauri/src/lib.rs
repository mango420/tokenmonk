//! Tokenmonk application core.

pub mod aggregate;
pub mod commands;
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
    let mut builder = tauri::Builder::default();

    // Must be registered first so a second launch exits before anything else starts.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // A second launch is ignored; showing the popup comes with the tray work.
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tokenmonk");
}
