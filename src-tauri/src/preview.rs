use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::backend::Backend;
use crate::drive::ProgressFn;
use crate::error::Result;
use crate::paths::AppPaths;
use crate::types::*;

const TEXT_LIMIT: u64 = 512 * 1024;
const MAX_RENDER_BYTES: u64 = 512 * 1024 * 1024;

/// Materialize everything the preview pane needs, caching binaries on disk.
///
/// Rendered files live next to the app's cache dir and are served to the
/// webview through Tauri's asset protocol; the webview never sees a Google
/// URL, so no token can leak into the DOM.
pub async fn prepare(backend: &Backend, paths: &AppPaths, id: &str) -> Result<PreviewInfo> {
    crate::backend::validate_id(id)?;
    let file = backend.metadata(id).await?;

    if file.is_folder() {
        return Ok(PreviewInfo {
            kind: "unsupported".into(),
            mime: file.mime_type,
            size: None,
            path: None,
            text: None,
            truncated: false,
            message: Some("文件夹没有可预览的内容".into()),
        });
    }

    if !file.can_download {
        return Ok(PreviewInfo {
            kind: "unsupported".into(),
            mime: file.mime_type,
            size: file.size,
            path: None,
            text: None,
            truncated: false,
            message: Some("该文件不允许下载".into()),
        });
    }

    if file.size.unwrap_or(0) > MAX_RENDER_BYTES {
        return Ok(PreviewInfo {
            kind: "unsupported".into(),
            mime: file.mime_type,
            size: file.size,
            path: None,
            text: None,
            truncated: false,
            message: Some("文件超过 512 MB，请下载后再打开".into()),
        });
    }

    let mime = file.mime_type.clone();
    match icon_type_for(&mime) {
        "text" => {
            let (text, truncated) = backend.text_preview(&file, TEXT_LIMIT).await?;
            Ok(PreviewInfo {
                kind: "text".into(),
                mime,
                size: file.size,
                path: None,
                text: Some(text),
                truncated,
                message: None,
            })
        }
        "doc" | "sheet" | "slide" | "draw" if pdf_export_available(&mime) => {
            let path = cache_path(paths, &file, "pdf");
            ensure_cached(backend, &file, &path, Some("application/pdf")).await?;
            Ok(binary_preview("pdf", mime, &file, path, None))
        }
        "image" => {
            let extension = extension_for_mime(&mime);
            let path = cache_path(paths, &file, extension);
            ensure_cached(backend, &file, &path, None).await?;
            Ok(binary_preview("image", mime, &file, path, None))
        }
        "pdf" => {
            let path = cache_path(paths, &file, "pdf");
            ensure_cached(backend, &file, &path, None).await?;
            Ok(binary_preview("pdf", mime, &file, path, None))
        }
        "video" => {
            let extension = extension_for_mime(&mime);
            let path = cache_path(paths, &file, extension);
            ensure_cached(backend, &file, &path, None).await?;
            Ok(binary_preview("video", mime, &file, path, None))
        }
        "audio" => {
            let extension = extension_for_mime(&mime);
            let path = cache_path(paths, &file, extension);
            ensure_cached(backend, &file, &path, None).await?;
            Ok(binary_preview("audio", mime, &file, path, None))
        }
        "office" => {
            let extension = extension_for_mime(&mime);
            let path = cache_path(paths, &file, extension);
            ensure_cached(backend, &file, &path, None).await?;
            Ok(binary_preview(
                "office",
                mime,
                &file,
                path,
                Some("Office 文档无法内嵌渲染，可用系统应用打开"),
            ))
        }
        "doc" | "sheet" | "slide" | "form" | "code" | "shortcut" => Ok(PreviewInfo {
            kind: "unsupported".into(),
            mime: mime.clone(),
            size: file.size,
            path: None,
            text: None,
            truncated: false,
            message: Some(format!("{mime} 暂不支持内嵌预览，可下载后用 Google 应用打开")),
        }),
        _ => Ok(PreviewInfo {
            kind: "unsupported".into(),
            mime: mime.clone(),
            size: file.size,
            path: None,
            text: None,
            truncated: false,
            message: Some(format!("{mime} 暂不支持预览")),
        }),
    }
}

fn binary_preview(kind: &str, mime: String, file: &FileMeta, path: PathBuf, message: Option<&str>) -> PreviewInfo {
    PreviewInfo {
        kind: kind.into(),
        mime,
        size: file.size,
        path: Some(path.to_string_lossy().to_string()),
        text: None,
        truncated: false,
        message: message.map(str::to_string),
    }
}

async fn ensure_cached(backend: &Backend, file: &FileMeta, path: &PathBuf, export_mime: Option<&str>) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let progress: ProgressFn = Arc::new(|_, _| {});
    let cancel = Arc::new(AtomicBool::new(false));
    match backend.download(file, path, export_mime, &progress, &cancel).await {
        Ok(_) => Ok(()),
        Err(error) => {
            let _ = tokio::fs::remove_file(path).await;
            Err(error)
        }
    }
}

fn cache_path(paths: &AppPaths, file: &FileMeta, extension: &str) -> PathBuf {
    let stamp = file
        .modified_time
        .as_deref()
        .map(|value| crate::secrets::sha256_hex(value.as_bytes()))
        .map(|digest| digest[..12].to_string())
        .unwrap_or_else(|| "unknown".into());
    let id: String = file
        .id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-' || *character == '_')
        .collect();
    paths
        .preview_dir()
        .join(format!("{}-{}-{}.{}", id, stamp, file.size.unwrap_or(0), extension))
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/svg+xml" => "svg",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        "audio/mpeg" => "mp3",
        "audio/mp4" => "m4a",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "application/pdf" => "pdf",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "application/vnd.ms-excel" | "application/vnd.ms-word" => "bin",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_names_are_stable_and_path_safe() {
        let paths = AppPaths::at(std::env::temp_dir().join("drivedesk-preview-test")).unwrap();
        let mut file = FileMeta {
            id: "../../etc/passwd".into(),
            name: "x".into(),
            mime_type: "image/png".into(),
            size: Some(10),
            modified_time: Some("2026-10-01T00:00:00.000Z".into()),
            created_time: None,
            owner: None,
            owner_email: None,
            parents: vec![],
            starred: false,
            trashed: false,
            md5: None,
            web_view_link: None,
            thumbnail_link: None,
            icon_type: "image".into(),
            shared: false,
            can_edit: true,
            can_download: true,
        };
        let path = cache_path(&paths, &file, "png");
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(!name.contains('/'));
        assert!(!name.contains(".."));
        assert!(name.ends_with(".png"));
        let again = cache_path(&paths, &file, "png");
        assert_eq!(path, again);
        file.modified_time = Some("2026-10-02T00:00:00.000Z".into());
        assert_ne!(cache_path(&paths, &file, "png"), path);
    }

    #[test]
    fn extension_mapping_covers_renderable_types() {
        assert_eq!(extension_for_mime("image/svg+xml"), "svg");
        assert_eq!(extension_for_mime("video/quicktime"), "mov");
        assert_eq!(extension_for_mime("application/octet-stream"), "bin");
    }
}
