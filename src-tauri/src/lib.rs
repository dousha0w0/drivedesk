pub mod auth;
pub mod backend;
pub mod commands;
pub mod drive;
pub mod error;
pub mod gdirect;
pub mod mock;
pub mod paths;
pub mod preview;
pub mod secrets;
pub mod settings;
pub mod state;
pub mod transfer;
pub mod types;

use tauri::Manager;

pub fn run() {
    let state = state::AppState::new().expect("failed to initialize application state");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::log_message,
            commands::app_info,
            commands::app_ready,
            commands::accounts_list,
            commands::account_login,
            commands::account_login_quick,
            commands::account_login_token,
            commands::account_switch,
            commands::account_logout,
            commands::quota_get,
            commands::files_list,
            commands::file_get,
            commands::file_path,
            commands::folder_create,
            commands::file_rename,
            commands::file_move,
            commands::file_copy,
            commands::file_trash,
            commands::file_star,
            commands::file_delete,
            commands::download_start,
            commands::upload_start,
            commands::transfers_list,
            commands::transfer_cancel,
            commands::transfer_retry,
            commands::transfers_clear_finished,
            commands::preview_prepare,
            commands::settings_get,
            commands::settings_update,
            commands::cache_clear,
            commands::gdirect_probe,
            commands::system_open_path,
            commands::system_open_url,
            commands::smoke_config,
            commands::smoke_fixture,
            commands::smoke_report,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            app.state::<state::AppState>().attach(handle.clone());

            // Safety net: if the frontend never signals readiness (panic, load
            // error), still reveal the window so the user sees the failure.
            let fallback = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                if let Some(window) = fallback.get_webview_window("main") {
                    if !window.is_visible().unwrap_or(true) {
                        let _ = window.show();
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running DriveDesk");
}
