use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, Result};
use crate::settings::SettingsPatch;
use crate::state::AppState;
use crate::types::*;

/// Webview diagnostics land on stderr, where a launcher (and the smoke run)
/// can see them; `console.log` alone is invisible outside devtools.
#[tauri::command]
pub async fn log_message(level: String, message: String) -> Result<()> {
    eprintln!("[webview:{level}] {message}");
    Ok(())
}

#[tauri::command]
pub async fn app_info(state: State<'_, AppState>) -> Result<AppInfo> {
    Ok(state.app_info().await)
}

/// The window is created hidden to avoid a white flash; the webview calls this
/// once the shell has painted.
#[tauri::command]
pub async fn app_ready(app: AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().ok();
        window.set_focus().ok();
    }
    Ok(())
}

#[tauri::command]
pub async fn accounts_list(state: State<'_, AppState>) -> Result<Vec<crate::auth::Account>> {
    Ok(state.backend.accounts().await)
}

#[tauri::command]
pub async fn account_login(state: State<'_, AppState>, client_json: Option<String>) -> Result<crate::auth::Account> {
    let path = client_json.map(PathBuf::from);
    state.backend.login(path).await
}

#[tauri::command]
pub async fn account_switch(state: State<'_, AppState>, email: String) -> Result<()> {
    state.backend.switch(&email).await
}

#[tauri::command]
pub async fn account_logout(state: State<'_, AppState>, email: String) -> Result<()> {
    state.backend.logout(&email).await
}

#[tauri::command]
pub async fn quota_get(state: State<'_, AppState>) -> Result<Quota> {
    state.backend.quota().await
}

#[tauri::command]
pub async fn files_list(state: State<'_, AppState>, request: ListRequest) -> Result<ListResponse> {
    state.backend.list(&request).await
}

#[tauri::command]
pub async fn file_get(state: State<'_, AppState>, id: String) -> Result<FileMeta> {
    state.backend.metadata(&id).await
}

#[tauri::command]
pub async fn file_path(state: State<'_, AppState>, id: String) -> Result<Vec<Crumb>> {
    state.backend.path(&id).await
}

#[tauri::command]
pub async fn folder_create(state: State<'_, AppState>, parent_id: String, name: String) -> Result<FileMeta> {
    state.backend.create_folder(&parent_id, &name).await
}

#[tauri::command]
pub async fn file_rename(state: State<'_, AppState>, id: String, name: String) -> Result<FileMeta> {
    state.backend.rename(&id, &name).await
}

#[tauri::command]
pub async fn file_move(state: State<'_, AppState>, id: String, new_parent: String) -> Result<FileMeta> {
    state.backend.move_file(&id, &new_parent).await
}

#[tauri::command]
pub async fn file_copy(state: State<'_, AppState>, id: String, parent: String) -> Result<FileMeta> {
    state.backend.copy(&id, &parent).await
}

#[tauri::command]
pub async fn file_trash(state: State<'_, AppState>, id: String, trashed: bool) -> Result<FileMeta> {
    state.backend.trash(&id, trashed).await
}

#[tauri::command]
pub async fn file_delete(state: State<'_, AppState>, id: String) -> Result<()> {
    state.backend.delete(&id).await
}

#[tauri::command]
pub async fn file_star(state: State<'_, AppState>, id: String, starred: bool) -> Result<FileMeta> {
    state.backend.star(&id, starred).await
}

#[tauri::command]
pub async fn download_start(
    state: State<'_, AppState>,
    ids: Vec<String>,
    dest_dir: Option<String>,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut files = Vec::with_capacity(ids.len());
    for id in &ids {
        files.push(state.backend.metadata(id).await?);
    }
    let dir = dest_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| state.settings.read().unwrap().download_dir.clone());
    state.transfers.enqueue_download(files, dir);
    Ok(())
}

#[tauri::command]
pub async fn upload_start(
    state: State<'_, AppState>,
    parent_id: String,
    paths: Vec<String>,
) -> Result<()> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    for path in &paths {
        if !path.is_file() {
            return Err(AppError::Invalid(format!("不是文件：{}", path.display())));
        }
    }
    state.transfers.enqueue_upload(parent_id, paths);
    Ok(())
}

#[tauri::command]
pub async fn transfers_list(state: State<'_, AppState>) -> Result<Vec<crate::transfer::Transfer>> {
    Ok(state.transfers.list())
}

#[tauri::command]
pub async fn transfer_cancel(state: State<'_, AppState>, id: String) -> Result<()> {
    state.transfers.cancel(&id);
    Ok(())
}

#[tauri::command]
pub async fn transfer_retry(state: State<'_, AppState>, id: String) -> Result<()> {
    state.transfers.retry(&id);
    Ok(())
}

#[tauri::command]
pub async fn transfers_clear_finished(state: State<'_, AppState>) -> Result<()> {
    state.transfers.clear_finished();
    Ok(())
}

#[tauri::command]
pub async fn preview_prepare(state: State<'_, AppState>, id: String) -> Result<PreviewInfo> {
    crate::preview::prepare(&state.backend, &state.paths, &id).await
}

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<crate::settings::Settings> {
    Ok(state.settings.read().unwrap().clone())
}

#[tauri::command]
pub async fn settings_update(
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<crate::settings::Settings> {
    let updated = {
        let mut settings = state.settings.write().unwrap();
        settings.apply(patch);
        settings.clone()
    };
    updated.save(&state.paths)?;
    state
        .transfers
        .set_limits(updated.download_concurrency, updated.upload_concurrency);
    Ok(updated)
}

#[tauri::command]
pub async fn cache_clear(state: State<'_, AppState>) -> Result<()> {
    let dir = state.paths.cache_dir.clone();
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(state.paths.preview_dir())?;
    Ok(())
}

#[tauri::command]
pub async fn gdirect_probe(path: Option<String>) -> Result<GdirectInfo> {
    Ok(crate::gdirect::probe(path.as_deref()))
}

#[tauri::command]
pub async fn system_open_path(path: String) -> Result<()> {
    crate::auth::open_external(&path)
}

#[tauri::command]
pub async fn system_open_url(url: String) -> Result<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(AppError::Invalid("只允许打开 http(s) 链接".into()));
    }
    crate::auth::open_external(&url)
}

/// Writes a throwaway upload fixture for the scripted smoke run. Only
/// callable while `DRIVEDESK_SMOKE=1`; the UI never invokes it.
#[tauri::command]
pub async fn smoke_fixture(state: State<'_, AppState>) -> Result<String> {
    if !state.smoke.enabled {
        return Err(AppError::Invalid("smoke mode is off".into()));
    }
    let path = state.paths.data_dir.join("smoke-upload.txt");
    std::fs::write(&path, b"DriveDesk smoke fixture\n")?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn smoke_config(state: State<'_, AppState>) -> Result<SmokeConfig> {
    Ok(state.smoke.clone())
}

/// Terminal step of the scripted UI smoke run: persist the report, print it for
/// the launcher, then quit. Only reachable when `DRIVEDESK_SMOKE=1`.
#[tauri::command]
pub async fn smoke_report(app: AppHandle, state: State<'_, AppState>, report: String) -> Result<()> {
    if !state.smoke.enabled {
        return Ok(());
    }
    if let Some(output) = &state.smoke.output {
        std::fs::write(output, &report)?;
    }
    println!("SMOKE_RESULT {report}");
    let hold = state.smoke.keep_open_ms;
    tauri::async_runtime::spawn(async move {
        if hold > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(hold)).await;
        }
        app.exit(0);
    });
    Ok(())
}
