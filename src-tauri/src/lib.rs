//! Toolpulse — dev tools version inspector.
//!
//! Startup wires together the settings/history paths, builds the shared
//! `AppState`, installs the menu-bar tray icon, spawns the notification
//! scheduler, and registers the IPC command handlers.

mod commands;
mod runner;
mod scheduler;
mod scheduler_state;
mod state;
mod tools;
mod tray;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use directories::ProjectDirs;

use crate::scheduler::AppStateShared;
use crate::state::AppState;
use crate::tools::History;

/// Resolve where to store user data (DB + settings).
///
/// On macOS this is `~/Library/Application Support/com.toolpulse.app/`.
fn data_dir() -> Option<PathBuf> {
    ProjectDirs::from("com", "toolpulse", "app").map(|d| d.data_dir().to_path_buf())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let dirs = data_dir().unwrap_or_else(|| PathBuf::from("./data"));
    let _ = fs::create_dir_all(&dirs);

    let db_path = dirs.join("toolpulse.db");
    let settings_path = dirs.join("settings.json");
    let history = match History::open(db_path) {
        Ok(h) => Some(h),
        Err(e) => {
            eprintln!("warning: failed to open history DB: {e}");
            None
        }
    };

    let app_state = Arc::new(AppState::new(history, settings_path.clone()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::check_all_tools,
            commands::check_tool,
            commands::get_latest_version,
            commands::list_installed_packages,
            commands::get_history,
            commands::get_settings,
            commands::save_settings,
            commands::dashboard_summary,
            commands::notify_outdated,
            commands::settings_path,
            commands::manage_tool,
            commands::manage_package,
            commands::cancel_run,
            commands::scan_projects,
            commands::scan_machine,
            commands::pick_folder,
            commands::scan_project_deps,
            commands::detect_ides,
            commands::open_folder,
            commands::open_in_ide,
            commands::trash_folder,
            commands::open_terminal,
            commands::scan_source_files,
            commands::check_for_update,
            commands::install_update,
            commands::system_info,
        ])
        .setup({
            let state = app_state.clone();
            move |app| {
                // Install the macOS menu-bar tray icon.
                tray::install(app.handle())?;

                // Spawn the background notification scheduler. It owns an
                // `Arc<AppStateShared>` so it can re-read settings, run scans,
                // and update the tray without touching Tauri's request-scoped
                // `State` wrapper.
                let shared = Arc::new(AppStateShared {
                    settings_path: state.settings_path.clone(),
                    inner: state.clone(),
                });
                scheduler::spawn(app.handle().clone(), shared);
                Ok(())
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
