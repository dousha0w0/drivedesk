use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::process::Command;

use crate::drive::{CancelToken, ProgressFn};
use crate::error::{AppError, Result};
use crate::types::{FileMeta, GdirectInfo};

/// Optional transport through a locally installed `gdirect` binary.
///
/// gdirect is a separate GPL-2.0 program with its own Google login and its own
/// route scanner (censored-network IP pinning). DriveDesk never links it: it
/// spawns the binary and reads its output, so the two licenses stay separate.
/// Because it authenticates independently, enabling this switches *transfers*
/// only — browsing still uses the DriveDesk account.
pub struct Gdirect {
    pub path: String,
    pub version: Option<String>,
}

pub fn probe(explicit: Option<&str>) -> GdirectInfo {
    let candidates: Vec<String> = match explicit {
        Some(path) if !path.trim().is_empty() => vec![path.trim().to_string()],
        _ => {
            let mut found = Vec::new();
            if let Some(paths) = std::env::var_os("PATH") {
                for dir in std::env::split_paths(&paths) {
                    for name in ["gdirect", "gdirect.exe"] {
                        let candidate = dir.join(name);
                        if candidate.is_file() {
                            found.push(candidate.to_string_lossy().to_string());
                        }
                    }
                }
            }
            if let Some(home) = dirs::home_dir() {
                let candidate = home.join(".local/bin/gdirect");
                if candidate.is_file() {
                    found.push(candidate.to_string_lossy().to_string());
                }
            }
            found
        }
    };

    for path in candidates {
        if let Ok(version) = run_version(&path) {
            return GdirectInfo { found: true, path: Some(path), version: Some(version) };
        }
    }
    GdirectInfo { found: false, path: None, version: None }
}

fn run_version(path: &str) -> Result<String> {
    let output = std::process::Command::new(path).arg("--version").output()?;
    if !output.status.success() {
        return Err(AppError::Message("gdirect --version failed".into()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn resolve(explicit: Option<&str>) -> Option<Gdirect> {
    let info = probe(explicit);
    info.path.map(|path| Gdirect { path, version: info.version })
}

impl Gdirect {
    pub async fn download(
        &self,
        file: &FileMeta,
        dest: &Path,
        export_mime: Option<&str>,
        connections: u8,
        progress: &ProgressFn,
        cancel: &CancelToken,
    ) -> Result<PathBuf> {
        if file.is_folder() {
            return Err(AppError::Message("gdirect 传输不支持文件夹，请先展开".into()));
        }
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut command = Command::new(&self.path);
        command
            .arg("download")
            .arg(&file.id)
            .arg("-o")
            .arg(dest)
            .arg("-j")
            .arg(connections.clamp(1, 16).to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(mime) = export_mime {
            command.arg("--export").arg(mime);
        }
        let mut child = command.spawn().map_err(|error| {
            AppError::Message(format!("无法启动 gdirect：{error}"))
        })?;

        // gdirect exposes no machine-readable progress, but it writes the file
        // as it goes, so the file size on disk is real progress.
        let total = file.size;
        let part = crate::drive::with_suffix(dest, ".part");
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill().await;
                return Err(AppError::Cancelled);
            }
            let received = tokio::fs::metadata(&part)
                .await
                .map(|meta| meta.len())
                .or_else(|_| std::fs::metadata(dest).map(|meta| meta.len()))
                .unwrap_or(0);
            progress(received, total);
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        progress(total.unwrap_or(received), total);
                        return Ok(dest.to_path_buf());
                    }
                    let mut stderr = String::new();
                    if let Some(mut pipe) = child.stderr.take() {
                        use tokio::io::AsyncReadExt;
                        let mut buffer = Vec::new();
                        let _ = pipe.read_to_end(&mut buffer).await;
                        stderr = String::from_utf8_lossy(&buffer).to_string();
                    }
                    return Err(AppError::Message(format!(
                        "gdirect 下载失败：{}",
                        stderr.lines().last().unwrap_or("未知错误")
                    )));
                }
                Ok(None) => tokio::time::sleep(Duration::from_millis(400)).await,
                Err(error) => return Err(AppError::Message(format!("gdirect 进程异常：{error}"))),
            }
        }
    }

    pub async fn upload(
        &self,
        local: &Path,
        parent: &str,
        name: &str,
        progress: &ProgressFn,
        cancel: &CancelToken,
    ) -> Result<String> {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Cancelled);
        }
        let total = std::fs::metadata(local).ok().map(|meta| meta.len());
        progress(0, total);
        let output = Command::new(&self.path)
            .arg("upload")
            .arg(local)
            .arg("--parent")
            .arg(parent)
            .arg("--name")
            .arg(name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|error| AppError::Message(format!("无法启动 gdirect：{error}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Message(format!(
                "gdirect 上传失败：{}",
                stderr.lines().last().unwrap_or("未知错误")
            )));
        }
        progress(total.unwrap_or(0), total);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let id = stdout
            .lines()
            .find_map(|line| line.split('\t').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Message("gdirect 没有返回文件 ID".into()))?;
        Ok(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_reports_not_found() {
        let info = probe(Some("/nonexistent/gdirect"));
        assert!(!info.found);
        assert!(info.path.is_none());
    }
}
