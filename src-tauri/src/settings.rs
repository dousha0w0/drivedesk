use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths::{default_download_dir, AppPaths};

/// Settings the backend owns. UI-only preferences (theme, palette, panel
/// widths) live in the webview's localStorage so they apply before first paint.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub download_dir: PathBuf,
    pub download_concurrency: u8,
    pub upload_concurrency: u8,
    pub gdirect_enabled: bool,
    pub gdirect_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: default_download_dir(),
            download_concurrency: 3,
            upload_concurrency: 2,
            gdirect_enabled: false,
            gdirect_path: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub download_dir: Option<PathBuf>,
    pub download_concurrency: Option<u8>,
    pub upload_concurrency: Option<u8>,
    pub gdirect_enabled: Option<bool>,
    pub gdirect_path: Option<String>,
}

impl Settings {
    pub fn load(paths: &AppPaths) -> Self {
        let file = paths.settings_file();
        std::fs::read_to_string(file)
            .ok()
            .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
            .map(Settings::sanitize)
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(paths.settings_file(), raw)?;
        Ok(())
    }

    pub fn sanitize(mut self) -> Self {
        self.download_concurrency = self.download_concurrency.clamp(1, 8);
        self.upload_concurrency = self.upload_concurrency.clamp(1, 8);
        if self.download_dir.as_os_str().is_empty() {
            self.download_dir = default_download_dir();
        }
        self
    }

    pub fn apply(&mut self, patch: SettingsPatch) {
        if let Some(value) = patch.download_dir {
            self.download_dir = value;
        }
        if let Some(value) = patch.download_concurrency {
            self.download_concurrency = value;
        }
        if let Some(value) = patch.upload_concurrency {
            self.upload_concurrency = value;
        }
        if let Some(value) = patch.gdirect_enabled {
            self.gdirect_enabled = value;
        }
        if patch.gdirect_path.is_some() {
            self.gdirect_path = patch.gdirect_path;
        }
        *self = self.clone().sanitize();
    }
}
