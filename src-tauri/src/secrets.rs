use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{AppError, Result};

const SERVICE: &str = "com.drivedesk.app.oauth";

/// Refresh-token storage.
///
/// The token is a long-lived Google credential, so it goes into the OS secret
/// store where one exists: Keychain on macOS, DPAPI on Windows, Secret Service
/// on Linux. Only when no store is available does it fall back to a 0600 file,
/// and that path is reported so the UI can warn instead of pretending.
pub struct SecretStore {
    fallback_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Keychain,
    Dpapi,
    SecretService,
    File,
}

impl StorageKind {
    pub fn label(self) -> &'static str {
        match self {
            StorageKind::Keychain => "macOS Keychain",
            StorageKind::Dpapi => "Windows DPAPI",
            StorageKind::SecretService => "Secret Service",
            StorageKind::File => "local file (0600)",
        }
    }
}

impl SecretStore {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { fallback_dir: data_dir.join("secrets") }
    }

    pub fn kind(&self) -> StorageKind {
        #[cfg(target_os = "macos")]
        {
            return StorageKind::Keychain;
        }
        #[cfg(target_os = "windows")]
        {
            return StorageKind::Dpapi;
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if which("secret-tool") {
                return StorageKind::SecretService;
            }
            return StorageKind::File;
        }
        #[allow(unreachable_code)]
        StorageKind::File
    }

    pub fn set(&self, account: &str, secret: &str) -> Result<()> {
        match self.kind() {
            StorageKind::Keychain => {
                // `security -U` updates in place, so a re-login does not leave
                // a stale duplicate behind.
                let status = Command::new("security")
                    .args(["add-generic-password", "-U", "-s", SERVICE, "-a", account, "-w", secret])
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .status()?;
                if !status.success() {
                    return Err(AppError::Message("Keychain refused the credential".into()));
                }
                Ok(())
            }
            StorageKind::SecretService => {
                let mut child = Command::new("secret-tool")
                    .args(["store", "--label", "DriveDesk", "service", SERVICE, "account", account])
                    .stdin(Stdio::piped())
                    .spawn()?;
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    stdin.write_all(secret.as_bytes())?;
                }
                let status = child.wait()?;
                if !status.success() {
                    return Err(AppError::Message("Secret Service refused the credential".into()));
                }
                Ok(())
            }
            StorageKind::Dpapi => dpapi_write(&self.file_for(account), secret),
            StorageKind::File => write_private(&self.file_for(account), secret),
        }
    }

    pub fn get(&self, account: &str) -> Result<Option<String>> {
        match self.kind() {
            StorageKind::Keychain => {
                let output = Command::new("security")
                    .args(["find-generic-password", "-s", SERVICE, "-a", account, "-w"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()?;
                if !output.status.success() {
                    return Ok(None);
                }
                Ok(Some(String::from_utf8_lossy(&output.stdout).trim_end_matches('\n').to_string()))
            }
            StorageKind::SecretService => {
                let output = Command::new("secret-tool")
                    .args(["lookup", "service", SERVICE, "account", account])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()?;
                if !output.status.success() {
                    return Ok(None);
                }
                let value = String::from_utf8_lossy(&output.stdout).to_string();
                Ok((!value.trim().is_empty()).then_some(value))
            }
            StorageKind::Dpapi => dpapi_read(&self.file_for(account)),
            StorageKind::File => match std::fs::read_to_string(self.file_for(account)) {
                Ok(value) => {
                    let trimmed = value.trim_end_matches('\n').to_string();
                    Ok((!trimmed.is_empty()).then_some(trimmed))
                }
                Err(_) => Ok(None),
            },
        }
    }

    pub fn delete(&self, account: &str) -> Result<()> {
        match self.kind() {
            StorageKind::Keychain => {
                let _ = Command::new("security")
                    .args(["delete-generic-password", "-s", SERVICE, "-a", account])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                Ok(())
            }
            StorageKind::SecretService => {
                let _ = Command::new("secret-tool")
                    .args(["clear", "service", SERVICE, "account", account])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                Ok(())
            }
            StorageKind::Dpapi | StorageKind::File => {
                let file = self.file_for(account);
                if file.exists() {
                    std::fs::remove_file(file)?;
                }
                Ok(())
            }
        }
    }

    fn file_for(&self, account: &str) -> PathBuf {
        let digest = sha256_hex(account.as_bytes());
        self.fallback_dir.join(format!("{}.secret", &digest[..32]))
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn write_private(path: &PathBuf, secret: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, secret)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn which(binary: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .any(|dir| dir.join(binary).is_file())
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn dpapi_write(path: &PathBuf, secret: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let script = "$b=[Text.Encoding]::UTF8.GetBytes($env:DRIVEDESK_SECRET); \
                  $e=[Security.Cryptography.ProtectedData]::Protect($b,$null,'CurrentUser'); \
                  [IO.File]::WriteAllBytes($env:DRIVEDESK_TARGET,$e)";
    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("DRIVEDESK_SECRET", secret)
        .env("DRIVEDESK_TARGET", path)
        .status()?;
    if !status.success() {
        return Err(AppError::Message("DPAPI encryption failed".into()));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn dpapi_read(path: &PathBuf) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let script = "$e=[IO.File]::ReadAllBytes($env:DRIVEDESK_TARGET); \
                  $b=[Security.Cryptography.ProtectedData]::Unprotect($e,$null,'CurrentUser'); \
                  [Console]::Out.Write([Text.Encoding]::UTF8.GetString($b))";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("DRIVEDESK_TARGET", path)
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

#[cfg(not(target_os = "windows"))]
fn dpapi_write(_path: &PathBuf, _secret: &str) -> Result<()> {
    Err(AppError::Message("DPAPI is only available on Windows".into()))
}

#[cfg(not(target_os = "windows"))]
fn dpapi_read(_path: &PathBuf) -> Result<Option<String>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_is_stable_and_does_not_leak_the_address() {
        let store = SecretStore::new(PathBuf::from("/tmp/drivedesk-secret-test"));
        let path = store.file_for("someone@example.com");
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(!name.contains("someone"));
        assert!(name.ends_with(".secret"));
        assert_eq!(path, store.file_for("someone@example.com"));
    }
}
