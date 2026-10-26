use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::auth::{Account, AuthStore};
use crate::drive::{CancelToken, DriveClient, ProgressFn};
use crate::error::{AppError, Result};
use crate::mock::MockDrive;
use crate::types::*;

pub struct Live {
    pub client: DriveClient,
    pub auth: Arc<AuthStore>,
}

pub struct Mock {
    pub drive: MockDrive,
}

/// The single seam between commands and "the cloud". Live talks to Google;
/// Mock serves a deterministic tree so the whole app runs without credentials.
pub enum Backend {
    Live(Live),
    Mock(Mock),
}

impl Backend {
    pub fn live(auth: Arc<AuthStore>) -> Result<Self> {
        Ok(Self::Live(Live { client: DriveClient::new(Arc::clone(&auth))?, auth }))
    }

    pub fn mock(upload_dir: PathBuf) -> Self {
        Self::Mock(Mock { drive: MockDrive::new(upload_dir) })
    }

    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock(_))
    }

    pub async fn accounts(&self) -> Vec<Account> {
        match self {
            Self::Live(live) => live.auth.accounts(),
            Self::Mock(mock) => mock.drive.accounts(),
        }
    }

    pub async fn login(&self, client_json: Option<PathBuf>) -> Result<Account> {
        match self {
            Self::Live(live) => live.auth.login(client_json).await,
            Self::Mock(mock) => Ok(mock.drive.accounts().remove(0)),
        }
    }

    pub async fn login_token(&self, token: String) -> Result<Account> {
        match self {
            Self::Live(live) => live.auth.login_token(token).await,
            Self::Mock(mock) => Ok(mock.drive.accounts().remove(0)),
        }
    }

    pub async fn login_builtin(&self) -> Result<Account> {
        match self {
            Self::Live(live) => live.auth.login_builtin().await,
            Self::Mock(mock) => Ok(mock.drive.accounts().remove(0)),
        }
    }

    pub async fn switch(&self, email: &str) -> Result<()> {
        match self {
            Self::Live(live) => live.auth.switch(email),
            Self::Mock(_) => Ok(()),
        }
    }

    pub async fn logout(&self, email: &str) -> Result<()> {
        match self {
            Self::Live(live) => live.auth.logout(email),
            Self::Mock(_) => Ok(()),
        }
    }

    pub async fn quota(&self) -> Result<Quota> {
        match self {
            Self::Live(live) => live.client.quota().await,
            Self::Mock(mock) => Ok(mock.drive.quota()),
        }
    }

    pub async fn list(&self, request: &ListRequest) -> Result<ListResponse> {
        match self {
            Self::Live(live) => live.client.list(request.clone()).await,
            Self::Mock(mock) => mock.drive.list(request),
        }
    }

    pub async fn metadata(&self, id: &str) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.metadata(id).await,
            Self::Mock(mock) => mock.drive.metadata(id),
        }
    }

    pub async fn path(&self, id: &str) -> Result<Vec<Crumb>> {
        match self {
            Self::Live(live) => live.client.path(id).await,
            Self::Mock(mock) => mock.drive.path(id),
        }
    }

    pub async fn create_folder(&self, parent: &str, name: &str) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.create_folder(parent, name).await,
            Self::Mock(mock) => mock.drive.create_folder(parent, name),
        }
    }

    pub async fn rename(&self, id: &str, name: &str) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.rename(id, name).await,
            Self::Mock(mock) => mock.drive.rename(id, name),
        }
    }

    pub async fn move_file(&self, id: &str, parent: &str) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.move_file(id, parent).await,
            Self::Mock(mock) => mock.drive.move_file(id, parent),
        }
    }

    pub async fn copy(&self, id: &str, parent: &str) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.copy(id, parent).await,
            Self::Mock(mock) => mock.drive.copy(id, parent),
        }
    }

    pub async fn trash(&self, id: &str, trashed: bool) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.trash(id, trashed).await,
            Self::Mock(mock) => mock.drive.trash(id, trashed),
        }
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        match self {
            Self::Live(live) => live.client.delete(id).await,
            Self::Mock(mock) => mock.drive.delete(id),
        }
    }

    pub async fn star(&self, id: &str, starred: bool) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.star(id, starred).await,
            Self::Mock(mock) => mock.drive.star(id, starred),
        }
    }

    pub async fn unique_name(&self, parent: &str, name: &str) -> Result<String> {
        match self {
            Self::Live(live) => live.client.unique_name(parent, name).await,
            Self::Mock(mock) => mock.drive.unique_name(parent, name),
        }
    }

    pub async fn download(
        &self,
        file: &FileMeta,
        dest: &Path,
        export_mime: Option<&str>,
        progress: &ProgressFn,
        cancel: &CancelToken,
    ) -> Result<PathBuf> {
        match self {
            Self::Live(live) => live.client.download_to(file, dest, export_mime, progress, cancel).await,
            Self::Mock(mock) => mock.drive.download_to(file, dest, export_mime, progress, cancel).await,
        }
    }

    pub async fn upload(
        &self,
        local: &Path,
        parent: &str,
        name: &str,
        progress: &ProgressFn,
        cancel: &CancelToken,
    ) -> Result<FileMeta> {
        match self {
            Self::Live(live) => live.client.upload_file(local, parent, name, progress, cancel).await,
            Self::Mock(mock) => mock.drive.upload_file(local, parent, name, progress, cancel).await,
        }
    }

    pub async fn text_preview(&self, file: &FileMeta, limit: u64) -> Result<(String, bool)> {
        match self {
            Self::Live(live) => live.client.text_preview(file, limit).await,
            Self::Mock(mock) => mock.drive.text_preview(file, limit).await,
        }
    }
}

pub fn default_export_mime(file: &FileMeta) -> Option<&'static str> {
    export_target_for(&file.mime_type).map(|(mime, _)| mime)
}

pub fn default_export_extension(file: &FileMeta) -> Option<&'static str> {
    export_target_for(&file.mime_type).map(|(_, extension)| extension)
}

/// Reject ids that cannot be a Drive id before they reach a URL.
pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 512 {
        return Err(AppError::Invalid("invalid file id".into()));
    }
    if !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')) {
        return Err(AppError::Invalid("invalid file id".into()));
    }
    Ok(())
}
