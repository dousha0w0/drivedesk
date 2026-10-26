use serde::{Deserialize, Serialize};

/// Everything the webview exchanges with Rust. Field names are camelCase on
/// both sides so no translation layer exists between the two.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size: Option<u64>,
    pub modified_time: Option<String>,
    pub created_time: Option<String>,
    pub owner: Option<String>,
    pub owner_email: Option<String>,
    pub parents: Vec<String>,
    pub starred: bool,
    pub trashed: bool,
    pub md5: Option<String>,
    pub web_view_link: Option<String>,
    pub thumbnail_link: Option<String>,
    /// Coarse kind used for icons and preview routing.
    pub icon_type: String,
    pub shared: bool,
    pub can_edit: bool,
    pub can_download: bool,
}

impl FileMeta {
    pub fn is_folder(&self) -> bool {
        self.mime_type == "application/vnd.google-apps.folder"
    }

    pub fn is_google_apps(&self) -> bool {
        self.mime_type.starts_with("application/vnd.google-apps.")
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListRequest {
    /// `folder` | `recent` | `starred` | `trash` | `shared` | `search`
    pub view: String,
    pub folder_id: Option<String>,
    pub query: Option<String>,
    pub page_token: Option<String>,
    pub page_size: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse {
    pub files: Vec<FileMeta>,
    pub next_page_token: Option<String>,
    pub total: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Crumb {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quota {
    pub limit: Option<u64>,
    pub usage: Option<u64>,
    pub drive_usage: Option<u64>,
    pub user: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub mock: bool,
    pub data_dir: String,
    pub cache_dir: String,
    pub default_download_dir: String,
    pub secret_storage: String,
    pub accounts: Vec<crate::auth::Account>,
    pub gdirect_found: bool,
    pub gdirect_path: Option<String>,
    pub gdirect_version: Option<String>,
    pub platform: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewInfo {
    /// `text` | `image` | `pdf` | `video` | `audio` | `office` | `unsupported`
    pub kind: String,
    pub mime: String,
    pub size: Option<u64>,
    /// Absolute path on disk; the frontend turns it into an `asset:` URL.
    pub path: Option<String>,
    /// Inline text for the text renderer.
    pub text: Option<String>,
    pub truncated: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GdirectInfo {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmokeConfig {
    pub enabled: bool,
    pub output: Option<String>,
    pub keep_open_ms: u64,
}

/// Derive the coarse icon/preview kind from a MIME type.
pub fn icon_type_for(mime: &str) -> &'static str {
    match mime {
        "application/vnd.google-apps.folder" => "folder",
        "application/vnd.google-apps.document" => "doc",
        "application/vnd.google-apps.spreadsheet" => "sheet",
        "application/vnd.google-apps.presentation" => "slide",
        "application/vnd.google-apps.drawing" => "draw",
        "application/vnd.google-apps.form" => "form",
        "application/vnd.google-apps.script" => "code",
        "application/vnd.google-apps.shortcut" => "shortcut",
        "application/pdf" => "pdf",
        _ if mime.starts_with("application/vnd.google-apps.") => "doc",
        _ if mime.starts_with("image/") => "image",
        _ if mime.starts_with("video/") => "video",
        _ if mime.starts_with("audio/") => "audio",
        _ if mime.starts_with("text/") => "text",
        "application/json" | "application/xml" | "application/javascript" | "application/x-yaml" => "text",
        "application/zip" | "application/x-tar" | "application/gzip" | "application/x-7z-compressed" => "archive",
        _ if mime.contains("officedocument") || mime.contains("msword") => "office",
        _ => "file",
    }
}

/// Export MIME + extension used when a Google-native document is downloaded.
pub fn export_target_for(mime: &str) -> Option<(&'static str, &'static str)> {
    match mime {
        "application/vnd.google-apps.document" => Some(("application/vnd.openxmlformats-officedocument.wordprocessingml.document", "docx")),
        "application/vnd.google-apps.spreadsheet" => Some(("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "xlsx")),
        "application/vnd.google-apps.presentation" => Some(("application/vnd.openxmlformats-officedocument.presentationml.presentation", "pptx")),
        "application/vnd.google-apps.drawing" => Some(("image/png", "png")),
        _ => None,
    }
}

/// Google-native documents that can be rendered as PDF for preview.
pub fn pdf_export_available(mime: &str) -> bool {
    matches!(
        mime,
        "application/vnd.google-apps.document"
            | "application/vnd.google-apps.spreadsheet"
            | "application/vnd.google-apps.presentation"
            | "application/vnd.google-apps.drawing"
    )
}
