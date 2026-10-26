use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;

use crate::backend::Backend;
use crate::gdirect::Gdirect;
use crate::settings::Settings;
use crate::drive::{disambiguate, CancelToken, ProgressFn};
use crate::error::{AppError, Result};
use crate::types::FileMeta;

pub const TRANSFER_EVENT: &str = "drive://transfer";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: String,
    /// `download` | `upload`
    pub kind: String,
    pub name: String,
    pub file_id: Option<String>,
    pub local_path: Option<String>,
    pub dest_dir: Option<String>,
    /// `queued` | `running` | `done` | `error` | `canceled`
    pub status: String,
    pub received: u64,
    pub total: Option<u64>,
    pub speed_bps: u64,
    pub error: Option<String>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub expanded_count: Option<u64>,
}

impl Transfer {
    fn new(id: String, kind: &str, name: String) -> Self {
        Self {
            id,
            kind: kind.into(),
            name,
            file_id: None,
            local_path: None,
            dest_dir: None,
            status: "queued".into(),
            received: 0,
            total: None,
            speed_bps: 0,
            error: None,
            started_at: None,
            finished_at: None,
            expanded_count: None,
        }
    }
}

#[derive(Clone)]
enum Job {
    Download { file: FileMeta, dest: PathBuf },
    Upload { local: PathBuf, parent: String, name: String },
}

pub struct Transfers {
    backend: Arc<Backend>,
    settings: Arc<RwLock<Settings>>,
    items: Arc<Mutex<Vec<Transfer>>>,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    cancels: Arc<Mutex<HashMap<String, CancelToken>>>,
    samples: Arc<Mutex<HashMap<String, (u64, Instant, Instant)>>>,
    app: Arc<RwLock<Option<AppHandle>>>,
    download_limit: Arc<RwLock<Arc<Semaphore>>>,
    upload_limit: Arc<RwLock<Arc<Semaphore>>>,
}

impl Transfers {
    pub fn new(
        backend: Arc<Backend>,
        settings: Arc<RwLock<Settings>>,
        download_concurrency: u8,
        upload_concurrency: u8,
    ) -> Self {
        Self {
            backend,
            settings,
            items: Arc::new(Mutex::new(Vec::new())),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            cancels: Arc::new(Mutex::new(HashMap::new())),
            samples: Arc::new(Mutex::new(HashMap::new())),
            app: Arc::new(RwLock::new(None)),
            download_limit: Arc::new(RwLock::new(Arc::new(Semaphore::new(download_concurrency.max(1) as usize)))),
            upload_limit: Arc::new(RwLock::new(Arc::new(Semaphore::new(upload_concurrency.max(1) as usize)))),
        }
    }

    pub fn attach(&self, app: AppHandle) {
        *self.app.write().unwrap() = Some(app);
    }

    pub fn set_limits(&self, download: u8, upload: u8) {
        *self.download_limit.write().unwrap() = Arc::new(Semaphore::new(download.clamp(1, 8) as usize));
        *self.upload_limit.write().unwrap() = Arc::new(Semaphore::new(upload.clamp(1, 8) as usize));
    }

    pub fn list(&self) -> Vec<Transfer> {
        self.items.lock().unwrap().clone()
    }

    pub fn clear_finished(&self) {
        self.items
            .lock()
            .unwrap()
            .retain(|item| !matches!(item.status.as_str(), "done" | "error" | "canceled"));
    }

    pub fn cancel(&self, id: &str) {
        if let Some(token) = self.cancels.lock().unwrap().get(id) {
            token.store(true, Ordering::Relaxed);
        }
        self.mutate(id, |item| {
            if item.status == "queued" {
                item.status = "canceled".into();
                item.finished_at = Some(now_ms());
            }
        });
        self.emit_for(id);
    }

    pub fn enqueue_download(self: &Arc<Self>, files: Vec<FileMeta>, dest_dir: PathBuf) {
        for file in files {
            let id = new_id();
            let mut item = Transfer::new(id.clone(), "download", file.name.clone());
            item.file_id = Some(file.id.clone());
            item.dest_dir = Some(dest_dir.to_string_lossy().to_string());
            if file.is_folder() {
                item.expanded_count = Some(0);
            }
            self.push(item);
            let dest = dest_dir.join(safe_file_name(&file.name));
            self.jobs.lock().unwrap().insert(id.clone(), Job::Download { file, dest });
            self.spawn(id);
        }
    }

    pub fn enqueue_upload(self: &Arc<Self>, parent: String, paths: Vec<PathBuf>) {
        for path in paths {
            let name = path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "upload".into());
            let id = new_id();
            let mut item = Transfer::new(id.clone(), "upload", name.clone());
            item.local_path = Some(path.to_string_lossy().to_string());
            item.total = std::fs::metadata(&path).ok().map(|meta| meta.len());
            self.push(item);
            self.jobs
                .lock()
                .unwrap()
                .insert(id.clone(), Job::Upload { local: path, parent: parent.clone(), name });
            self.spawn(id);
        }
    }

    pub fn retry(self: &Arc<Self>, id: &str) {
        let job = self.jobs.lock().unwrap().get(id).cloned();
        let Some(_job) = job else { return };
        self.mutate(id, |item| {
            item.status = "queued".into();
            item.received = 0;
            item.speed_bps = 0;
            item.error = None;
            item.started_at = None;
            item.finished_at = None;
        });
        self.samples.lock().unwrap().remove(id);
        self.emit_for(id);
        self.spawn(id.to_string());
    }

    fn push(&self, item: Transfer) {
        self.items.lock().unwrap().push(item.clone());
        self.emit(&item);
    }

    fn spawn(self: &Arc<Self>, id: String) {
        let manager = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            manager.run(id).await;
        });
    }

    async fn run(self: Arc<Self>, id: String) {
        let Some(job) = self.jobs.lock().unwrap().get(&id).cloned() else { return };
        let token: CancelToken = Arc::new(AtomicBool::new(false));
        self.cancels.lock().unwrap().insert(id.clone(), Arc::clone(&token));

        let limit = match job {
            Job::Download { .. } => Arc::clone(&self.download_limit.read().unwrap()),
            Job::Upload { .. } => Arc::clone(&self.upload_limit.read().unwrap()),
        };
        let _permit = match limit.acquire().await {
            Ok(permit) => permit,
            Err(_) => return,
        };

        if token.load(Ordering::Relaxed) {
            return;
        }

        self.mutate(&id, |item| {
            item.status = "running".into();
            item.started_at = Some(now_ms());
        });
        self.emit_for(&id);

        let result = match job {
            Job::Download { file, dest } => self.clone().run_download(&id, file, dest, &token).await,
            Job::Upload { local, parent, name } => self.clone().run_upload(&id, local, parent, name, &token).await,
        };

        match result {
            Ok(()) => self.mutate(&id, |item| {
                item.status = "done".into();
                item.speed_bps = 0;
                item.finished_at = Some(now_ms());
                if item.total.is_some() {
                    item.received = item.total.unwrap_or(item.received);
                }
            }),
            Err(AppError::Cancelled) => self.mutate(&id, |item| {
                item.status = "canceled".into();
                item.speed_bps = 0;
                item.finished_at = Some(now_ms());
            }),
            Err(error) => {
                eprintln!("transfer {id} failed: {error}");
                self.mutate(&id, |item| {
                    item.status = "error".into();
                    item.speed_bps = 0;
                    item.error = Some(error.to_string());
                    item.finished_at = Some(now_ms());
                })
            }
        }
        self.samples.lock().unwrap().remove(&id);
        self.cancels.lock().unwrap().remove(&id);
        self.emit_for(&id);
    }

    async fn run_download(
        self: Arc<Self>,
        id: &str,
        file: FileMeta,
        dest: PathBuf,
        token: &CancelToken,
    ) -> Result<()> {
        let backend = Arc::clone(&self.backend);

        if file.is_folder() {
            let mut flat: Vec<(FileMeta, String)> = Vec::new();
            let mut stack = vec![(file.id.clone(), String::new())];
            let mut guard = 0usize;
            while let Some((folder_id, prefix)) = stack.pop() {
                guard += 1;
                if guard > 400 {
                    return Err(AppError::Message("文件夹层级过深，已停止展开".into()));
                }
                let mut page_token = None;
                loop {
                    let response = backend
                        .list(&crate::types::ListRequest {
                            view: "folder".into(),
                            folder_id: Some(folder_id.clone()),
                            page_token: page_token.clone(),
                            page_size: Some(1000),
                            ..Default::default()
                        })
                        .await?;
                    for child in response.files {
                        if child.is_folder() {
                            stack.push((child.id.clone(), format!("{prefix}{}/", safe_file_name(&child.name))));
                        } else {
                            flat.push((child.clone(), format!("{prefix}{}", safe_file_name(&child.name))));
                        }
                    }
                    page_token = response.next_page_token;
                    if page_token.is_none() {
                        break;
                    }
                }
            }

            let total_files = flat.len() as u64;
            let total_bytes: u64 = flat.iter().filter_map(|(file, _)| file.size).sum();
            self.mutate(id, |item| {
                item.expanded_count = Some(total_files);
                item.total = Some(total_bytes.max(1));
            });
            self.emit_for(id);

            let mut finished_bytes: u64 = 0;
            for (child, relative) in flat {
                if token.load(Ordering::Relaxed) {
                    return Err(AppError::Cancelled);
                }
                let target = dest.join(&relative);
                if let Some(parent) = target.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                let base = finished_bytes;
                let manager = Arc::clone(&self);
                let id_owned = id.to_string();
                let progress: ProgressFn = Arc::new(move |received, _| {
                    manager.tick(&id_owned, base + received, None);
                });
                if let Err(error) = self
                    .fetch(&backend, &child, &target, &progress, token)
                    .await
                {
                    match error {
                        AppError::Cancelled => return Err(AppError::Cancelled),
                        other => eprintln!("skipping {}: {other}", child.name),
                    }
                }
                finished_bytes += child.size.unwrap_or(0);
            }
            return Ok(());
        }

        let manager = Arc::clone(&self);
        let id_owned = id.to_string();
        let progress: ProgressFn = Arc::new(move |received, total| {
            manager.tick(&id_owned, received, total);
        });

        // A retry must keep writing the same file (so `.part` still resumes);
        // a fresh download avoids clobbering an unrelated file of that name.
        let part = crate::drive::with_suffix(&dest, ".part");
        let target = match (dest.exists(), part.exists(), dest.file_name()) {
            (true, false, Some(name)) => unique_local_path(
                dest.parent().unwrap_or_else(|| Path::new(".")),
                &name.to_string_lossy(),
            ),
            _ => dest,
        };
        self.fetch(&backend, &file, &target, &progress, token).await?;
        Ok(())
    }

    async fn run_upload(
        self: Arc<Self>,
        id: &str,
        local: PathBuf,
        parent: String,
        name: String,
        token: &CancelToken,
    ) -> Result<()> {
        let backend = Arc::clone(&self.backend);
        let available = backend.unique_name(&parent, &name).await.unwrap_or(name);
        let manager = Arc::clone(&self);
        let id_owned = id.to_string();
        let progress: ProgressFn = Arc::new(move |sent, total| {
            manager.tick(&id_owned, sent, total);
        });
        match self.gdirect() {
            Some((binary, _)) => {
                let file_id = binary.upload(&local, &parent, &available, &progress, token).await?;
                self.mutate(id, |item| item.file_id = Some(file_id));
            }
            None => {
                let meta = backend.upload(&local, &parent, &available, &progress, token).await?;
                self.mutate(id, |item| {
                    item.file_id = Some(meta.id.clone());
                    item.name = meta.name.clone();
                });
            }
        }
        Ok(())
    }

    fn mutate(&self, id: &str, update: impl FnOnce(&mut Transfer)) {
        let mut items = self.items.lock().unwrap();
        if let Some(item) = items.iter_mut().find(|item| item.id == id) {
            update(item);
        }
    }

    fn tick(&self, id: &str, received: u64, total: Option<u64>) {
        let speed = {
            let mut samples = self.samples.lock().unwrap();
            let entry = samples.entry(id.to_string()).or_insert((0, Instant::now(), Instant::now()));
            let (last_bytes, last_at, last_emit) = *entry;
            let now = Instant::now();
            let elapsed = now.duration_since(last_at).as_secs_f64().max(0.001);
            let speed = ((received.saturating_sub(last_bytes)) as f64 / elapsed) as u64;
            let should_emit = now.duration_since(last_emit) >= Duration::from_millis(120) || received == total.unwrap_or(u64::MAX);
            if should_emit {
                *entry = (received, now, now);
            } else {
                entry.0 = received;
            }
            (speed, should_emit)
        };

        self.mutate(id, |item| {
            item.received = received;
            if total.is_some() {
                item.total = total;
            }
            item.speed_bps = speed.0;
        });
        if speed.1 {
            self.emit_for(id);
        }
    }

    fn emit_for(&self, id: &str) {
        let snapshot = self
            .items
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.id == id)
            .cloned();
        if let Some(item) = snapshot {
            self.emit(&item);
        }
    }

    async fn fetch(
        &self,
        backend: &Backend,
        file: &FileMeta,
        dest: &Path,
        progress: &ProgressFn,
        token: &CancelToken,
    ) -> Result<()> {
        match self.gdirect() {
            Some((binary, connections)) => binary
                .download(file, dest, None, connections, progress, token)
                .await
                .map(|_| ()),
            None => backend
                .download(file, dest, None, progress, token)
                .await
                .map(|_| ()),
        }
    }

    /// Active gdirect transport plus its connection count, when the user has
    /// enabled it and the binary probes clean.
    fn gdirect(&self) -> Option<(Gdirect, u8)> {
        let (enabled, path, connections) = {
            let settings = self.settings.read().unwrap();
            (
                settings.gdirect_enabled,
                settings.gdirect_path.clone(),
                settings.download_concurrency.max(1),
            )
        };
        if !enabled {
            return None;
        }
        crate::gdirect::resolve(path.as_deref()).map(|binary| (binary, connections))
    }

    fn emit(&self, item: &Transfer) {
        if let Some(app) = self.app.read().unwrap().as_ref() {
            let _ = app.emit(TRANSFER_EVENT, item);
        }
    }
}

fn new_id() -> String {
    use rand::Rng;
    let suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    format!("t-{}-{suffix}", now_ms())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Drive names may contain `/`, `:` and other characters that are illegal or
/// path-significant locally; replace them instead of failing the transfer.
pub fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim_matches('.').trim();
    if trimmed.is_empty() {
        "untitled".into()
    } else {
        trimmed.to_string()
    }
}

/// Locally unique destination inside `dir`.
pub fn unique_local_path(dir: &Path, name: &str) -> PathBuf {
    let taken: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    let name = safe_file_name(name);
    dir.join(disambiguate(&name, &taken))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_reserved_characters() {
        assert_eq!(safe_file_name("a/b:c*?.txt"), "a_b_c__.txt");
        assert_eq!(safe_file_name("..."), "untitled");
        assert_eq!(safe_file_name("正常名称.pdf"), "正常名称.pdf");
    }

    #[test]
    fn local_conflicts_get_a_counter_suffix() {
        let dir = std::env::temp_dir().join(format!("drivedesk-conflict-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("report.pdf"), b"x").unwrap();
        let path = unique_local_path(&dir, "report.pdf");
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "report (2).pdf");
        std::fs::remove_dir_all(&dir).ok();
    }
}
