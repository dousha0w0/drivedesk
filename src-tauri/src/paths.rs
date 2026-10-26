use std::path::PathBuf;

use crate::error::{AppError, Result};

/// All on-disk locations the app owns. Derived from the OS data/cache dirs so
/// the same binary works from a dev build and a bundled .app.
#[derive(Clone, Debug)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| AppError::Message("cannot resolve the application data directory".into()))?
            .join("DriveDesk");
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| AppError::Message("cannot resolve the cache directory".into()))?
            .join("DriveDesk");
        let paths = Self { data_dir, cache_dir };
        paths.ensure()?;
        Ok(paths)
    }

    /// Test/CI construction: keep every artifact inside one throwaway root.
    pub fn at(root: PathBuf) -> Result<Self> {
        let paths = Self { data_dir: root.join("data"), cache_dir: root.join("cache") };
        paths.ensure()?;
        Ok(paths)
    }

    fn ensure(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(self.preview_dir())?;
        Ok(())
    }

    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    pub fn accounts_file(&self) -> PathBuf {
        self.data_dir.join("accounts.json")
    }

    pub fn client_file(&self) -> PathBuf {
        self.data_dir.join("oauth-client.json")
    }

    pub fn preview_dir(&self) -> PathBuf {
        self.cache_dir.join("preview")
    }

    pub fn thumb_dir(&self) -> PathBuf {
        self.cache_dir.join("thumb")
    }
}

/// Default destination for downloaded files.
pub fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DriveDesk")
}
