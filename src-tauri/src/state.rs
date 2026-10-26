use std::sync::{Arc, RwLock};
use std::time::Instant;

use tauri::AppHandle;

use crate::backend::Backend;
use crate::error::Result;
use crate::paths::{default_download_dir, AppPaths};
use crate::settings::Settings;
use crate::transfer::Transfers;
use crate::types::{AppInfo, SmokeConfig};

pub struct AppState {
    pub paths: AppPaths,
    pub backend: Arc<Backend>,
    pub transfers: Arc<Transfers>,
    pub settings: Arc<RwLock<Settings>>,
    pub smoke: SmokeConfig,
    pub app: RwLock<Option<AppHandle>>,
    pub started_at: Instant,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let paths = AppPaths::resolve()?;
        let mock = std::env::var("DRIVEDESK_MOCK").map(|value| value == "1").unwrap_or(false);

        let backend = if mock {
            Arc::new(Backend::mock(paths.data_dir.join("mock-uploads")))
        } else {
            let auth = Arc::new(crate::auth::AuthStore::new(paths.clone()));
            Arc::new(Backend::live(auth)?)
        };

        let settings = Arc::new(RwLock::new(Settings::load(&paths)));
        let (download_concurrency, upload_concurrency) = {
            let settings = settings.read().unwrap();
            (settings.download_concurrency, settings.upload_concurrency)
        };
        let transfers = Arc::new(Transfers::new(
            Arc::clone(&backend),
            Arc::clone(&settings),
            download_concurrency,
            upload_concurrency,
        ));
        let smoke = SmokeConfig {
            enabled: std::env::var("DRIVEDESK_SMOKE").map(|value| value == "1").unwrap_or(false),
            output: std::env::var("DRIVEDESK_SMOKE_OUT").ok(),
            keep_open_ms: std::env::var("DRIVEDESK_SMOKE_HOLD")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
        };

        Ok(Self {
            paths,
            backend,
            transfers,
            settings,
            smoke,
            app: RwLock::new(None),
            started_at: Instant::now(),
        })
    }

    pub fn attach(&self, app: AppHandle) {
        self.transfers.attach(app.clone());
        *self.app.write().unwrap() = Some(app);
    }

    pub fn mock(&self) -> bool {
        self.backend.is_mock()
    }

    pub async fn app_info(&self) -> AppInfo {
        let configured_path = self.settings.read().unwrap().gdirect_path.clone();
        let gdirect = crate::gdirect::probe(configured_path.as_deref());
        let accounts = self.backend.accounts().await;
        AppInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            mock: self.mock(),
            data_dir: self.paths.data_dir.to_string_lossy().to_string(),
            cache_dir: self.paths.cache_dir.to_string_lossy().to_string(),
            default_download_dir: default_download_dir().to_string_lossy().to_string(),
            secret_storage: self.secret_storage(),
            accounts,
            builtin_client: crate::auth::AuthStore::has_builtin_client(),
            gdirect_found: gdirect.found,
            gdirect_path: gdirect.path,
            gdirect_version: gdirect.version,
            platform: std::env::consts::OS.to_string(),
        }
    }

    fn secret_storage(&self) -> String {
        match self.backend.as_ref() {
            Backend::Live(live) => live.auth.storage_label().to_string(),
            Backend::Mock(_) => "演示模式（不存储凭据）".to_string(),
        }
    }
}
