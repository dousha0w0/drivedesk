use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::auth::AuthStore;
use crate::error::{AppError, Result};
use crate::types::*;

const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD_API: &str = "https://www.googleapis.com/upload/drive/v3";
const LIST_FIELDS: &str = "nextPageToken,files(id,name,mimeType,size,modifiedTime,createdTime,owners(emailAddress,displayName),parents,starred,trashed,md5Checksum,webViewLink,thumbnailLink,shared,capabilities(canEdit,canDownload))";
const FILE_FIELDS: &str = "id,name,mimeType,size,modifiedTime,createdTime,owners(emailAddress,displayName),parents,starred,trashed,md5Checksum,webViewLink,thumbnailLink,shared,capabilities(canEdit,canDownload)";
const CHUNK: usize = 8 * 1024 * 1024;
const MAX_TEXT_PREVIEW: u64 = 512 * 1024;

pub type ProgressFn = Arc<dyn Fn(u64, Option<u64>) + Send + Sync>;
pub type CancelToken = Arc<AtomicBool>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOwner {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    email_address: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCapabilities {
    #[serde(default)]
    can_edit: bool,
    #[serde(default)]
    can_download: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    modified_time: Option<String>,
    #[serde(default)]
    created_time: Option<String>,
    #[serde(default)]
    owners: Option<Vec<RawOwner>>,
    #[serde(default)]
    parents: Option<Vec<String>>,
    #[serde(default)]
    starred: Option<bool>,
    #[serde(default)]
    trashed: Option<bool>,
    #[serde(default)]
    md5_checksum: Option<String>,
    #[serde(default)]
    web_view_link: Option<String>,
    #[serde(default)]
    thumbnail_link: Option<String>,
    #[serde(default)]
    shared: Option<bool>,
    #[serde(default)]
    capabilities: Option<RawCapabilities>,
}

impl RawFile {
    fn into_meta(self) -> FileMeta {
        let mime = self.mime_type.unwrap_or_else(|| "application/octet-stream".into());
        let owner = self.owners.as_ref().and_then(|owners| owners.first());
        FileMeta {
            icon_type: icon_type_for(&mime).to_string(),
            id: self.id,
            name: self.name.unwrap_or_else(|| "未命名".into()),
            mime_type: mime,
            size: self.size.and_then(|value| value.parse().ok()),
            modified_time: self.modified_time,
            created_time: self.created_time,
            owner: owner.and_then(|owner| owner.display_name.clone()),
            owner_email: owner.and_then(|owner| owner.email_address.clone()),
            parents: self.parents.unwrap_or_default(),
            starred: self.starred.unwrap_or(false),
            trashed: self.trashed.unwrap_or(false),
            md5: self.md5_checksum,
            web_view_link: self.web_view_link,
            thumbnail_link: self.thumbnail_link,
            shared: self.shared.unwrap_or(false),
            can_edit: self.capabilities.as_ref().map(|caps| caps.can_edit).unwrap_or(true),
            can_download: self.capabilities.as_ref().map(|caps| caps.can_download).unwrap_or(true),
        }
    }
}

pub struct DriveClient {
    http: reqwest::Client,
    auth: Arc<AuthStore>,
}

impl DriveClient {
    pub fn new(auth: Arc<AuthStore>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .user_agent("DriveDesk/0.1 (+https://github.com/drivedesk)")
            .build()?;
        Ok(Self { http, auth })
    }

    async fn token(&self, force_refresh: bool) -> Result<String> {
        let email = self.auth.default_email()?;
        if force_refresh {
            self.auth.invalidate(&email);
        }
        self.auth.access_token(&email).await
    }

    /// Authenticated JSON request with one forced-refresh retry and bounded
    /// backoff for Google's rate limiting.
    async fn send_json(
        &self,
        method: reqwest::Method,
        url: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value> {
        let mut refreshed = false;
        let mut attempt = 0u32;
        loop {
            let token = self.token(false).await?;
            let mut request = self
                .http
                .request(method.clone(), url)
                .bearer_auth(&token)
                .query(query);
            if let Some(payload) = &body {
                request = request.json(payload);
            }
            let response = match request.send().await {
                Ok(response) => response,
                Err(error) if attempt < 3 => {
                    attempt += 1;
                    tokio::time::sleep(backoff(attempt)).await;
                    eprintln!("retrying {url} after transport error: {error}");
                    continue;
                }
                Err(error) => return Err(error.into()),
            };

            let status = response.status();
            if status.as_u16() == 401 && !refreshed {
                refreshed = true;
                if let Ok(email) = self.auth.default_email() {
                    self.auth.invalidate(&email);
                }
                continue;
            }
            if matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504) && attempt < 3 {
                attempt += 1;
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs);
                tokio::time::sleep(retry_after.unwrap_or_else(|| backoff(attempt))).await;
                continue;
            }

            let text = response.text().await?;
            if !status.is_success() {
                return Err(AppError::Api {
                    status: status.as_u16(),
                    message: api_message(&text),
                });
            }
            if text.trim().is_empty() {
                return Ok(Value::Null);
            }
            return Ok(serde_json::from_str(&text)?);
        }
    }

    pub async fn accounts(&self) -> Vec<crate::auth::Account> {
        self.auth.accounts()
    }

    pub async fn list(&self, request: ListRequest) -> Result<ListResponse> {
        let page_size = request.page_size.unwrap_or(200).clamp(1, 1000);
        let sort_by = request.sort_by.clone().unwrap_or_else(|| "name".into());
        let sort_dir = request.sort_dir.clone().unwrap_or_else(|| "asc".into());

        let order_by = match (sort_by.as_str(), sort_dir.as_str()) {
            ("name", "desc") => Some("folder,name_natural desc"),
            ("name", _) => Some("folder,name_natural"),
            ("modified", "asc") => Some("folder,modifiedTime"),
            ("modified", _) => Some("folder,modifiedTime desc"),
            _ => None,
        };

        let mut query = vec![
            ("q".to_string(), build_query(&request)),
            ("fields".to_string(), LIST_FIELDS.to_string()),
            ("pageSize".to_string(), page_size.to_string()),
            ("supportsAllDrives".to_string(), "true".to_string()),
            ("includeItemsFromAllDrives".to_string(), "true".to_string()),
            ("spaces".to_string(), "drive".to_string()),
        ];
        if let Some(order) = order_by {
            query.push(("orderBy".into(), order.into()));
        }
        if let Some(token) = request.page_token.clone().filter(|token| !token.is_empty()) {
            query.push(("pageToken".into(), token));
        }

        let value = self
            .send_json(reqwest::Method::GET, &format!("{API}/files"), &query, None)
            .await?;
        let mut files: Vec<FileMeta> = value["files"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| serde_json::from_value::<RawFile>(item.clone()).ok())
                    .map(RawFile::into_meta)
                    .collect()
            })
            .unwrap_or_default();

        // Drive cannot order by size/type; order them locally, folders first,
        // so the table's sort arrows always mean what they show.
        if matches!(sort_by.as_str(), "size" | "type") {
            sort_files(&mut files, &sort_by, &sort_dir);
        }

        Ok(ListResponse {
            files,
            next_page_token: value["nextPageToken"].as_str().map(str::to_string),
            total: None,
        })
    }

    pub async fn metadata(&self, id: &str) -> Result<FileMeta> {
        let value = self
            .send_json(
                reqwest::Method::GET,
                &format!("{API}/files/{}", encode(id)),
                &[
                    ("fields".into(), FILE_FIELDS.into()),
                    ("supportsAllDrives".into(), "true".into()),
                ],
                None,
            )
            .await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    pub async fn path(&self, id: &str) -> Result<Vec<Crumb>> {
        if id == "root" {
            return Ok(vec![Crumb { id: "root".into(), name: "我的云盘".into() }]);
        }
        let mut crumbs = Vec::new();
        let mut cursor = id.to_string();
        for _ in 0..64 {
            let meta = self.metadata(&cursor).await?;
            crumbs.push(Crumb { id: meta.id.clone(), name: meta.name.clone() });
            match meta.parents.first() {
                Some(parent) => cursor = parent.clone(),
                None => break,
            }
        }
        crumbs.push(Crumb { id: "root".into(), name: "我的云盘".into() });
        crumbs.reverse();
        Ok(crumbs)
    }

    pub async fn create_folder(&self, parent: &str, name: &str) -> Result<FileMeta> {
        let value = self
            .send_json(
                reqwest::Method::POST,
                &format!("{API}/files"),
                &[("fields".into(), FILE_FIELDS.into()), ("supportsAllDrives".into(), "true".into())],
                Some(json!({
                    "name": name,
                    "mimeType": "application/vnd.google-apps.folder",
                    "parents": [parent],
                })),
            )
            .await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    pub async fn rename(&self, id: &str, name: &str) -> Result<FileMeta> {
        let value = self
            .send_json(
                reqwest::Method::PATCH,
                &format!("{API}/files/{}", encode(id)),
                &[("fields".into(), FILE_FIELDS.into()), ("supportsAllDrives".into(), "true".into())],
                Some(json!({ "name": name })),
            )
            .await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    pub async fn move_file(&self, id: &str, new_parent: &str) -> Result<FileMeta> {
        let meta = self.metadata(id).await?;
        let previous = meta.parents.join(",");
        let value = self
            .send_json(
                reqwest::Method::PATCH,
                &format!("{API}/files/{}", encode(id)),
                &[
                    ("addParents".into(), new_parent.into()),
                    ("removeParents".into(), previous),
                    ("fields".into(), FILE_FIELDS.into()),
                    ("supportsAllDrives".into(), "true".into()),
                ],
                Some(json!({})),
            )
            .await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    pub async fn copy(&self, id: &str, parent: &str) -> Result<FileMeta> {
        let source = self.metadata(id).await?;
        let value = self
            .send_json(
                reqwest::Method::POST,
                &format!("{API}/files/{}/copy", encode(id)),
                &[("fields".into(), FILE_FIELDS.into()), ("supportsAllDrives".into(), "true".into())],
                Some(json!({
                    "name": format!("{} 的副本", source.name),
                    "parents": [parent],
                })),
            )
            .await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    pub async fn trash(&self, id: &str, trashed: bool) -> Result<FileMeta> {
        let value = self
            .send_json(
                reqwest::Method::PATCH,
                &format!("{API}/files/{}", encode(id)),
                &[("fields".into(), FILE_FIELDS.into()), ("supportsAllDrives".into(), "true".into())],
                Some(json!({ "trashed": trashed })),
            )
            .await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    /// Permanent delete. Only meaningful for items already in the trash.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.send_json(
            reqwest::Method::DELETE,
            &format!("{API}/files/{}", encode(id)),
            &[("supportsAllDrives".into(), "true".into())],
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn star(&self, id: &str, starred: bool) -> Result<FileMeta> {
        let value = self
            .send_json(
                reqwest::Method::PATCH,
                &format!("{API}/files/{}", encode(id)),
                &[("fields".into(), FILE_FIELDS.into()), ("supportsAllDrives".into(), "true".into())],
                Some(json!({ "starred": starred })),
            )
            .await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    pub async fn quota(&self) -> Result<Quota> {
        let value = self
            .send_json(
                reqwest::Method::GET,
                &format!("{API}/about"),
                &[("fields".into(), "storageQuota,user(emailAddress)".into())],
                None,
            )
            .await?;
        Ok(Quota {
            limit: value["storageQuota"]["limit"].as_str().and_then(|v| v.parse().ok()),
            usage: value["storageQuota"]["usage"].as_str().and_then(|v| v.parse().ok()),
            drive_usage: value["storageQuota"]["usageInDrive"].as_str().and_then(|v| v.parse().ok()),
            user: value["user"]["emailAddress"].as_str().map(str::to_string),
        })
    }

    pub async fn unique_name(&self, parent: &str, name: &str) -> Result<String> {
        let escaped = escape_query(name);
        let query = vec![
            ("q".to_string(), format!("'{parent}' in parents and name = '{escaped}' and trashed = false")),
            ("fields".to_string(), "files(name)".to_string()),
            ("pageSize".to_string(), "1000".to_string()),
        ];
        let value = self
            .send_json(reqwest::Method::GET, &format!("{API}/files"), &query, None)
            .await?;
        let taken: Vec<String> = value["files"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["name"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        Ok(disambiguate(name, &taken))
    }

    /// Stream a file to `dest`, resuming from a leftover `.part` file.
    pub async fn download_to(
        &self,
        file: &FileMeta,
        dest: &Path,
        export_mime: Option<&str>,
        progress: &ProgressFn,
        cancel: &CancelToken,
    ) -> Result<PathBuf> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let part = with_suffix(dest, ".part");
        let mut existing = tokio::fs::metadata(&part).await.map(|meta| meta.len()).unwrap_or(0);

        let url = match export_mime {
            Some(mime_type) => format!(
                "{API}/files/{}/export?mimeType={}",
                encode(&file.id),
                encode(mime_type)
            ),
            None => format!("{API}/files/{}?alt=media&supportsAllDrives=true", encode(&file.id)),
        };

        let mut refreshed = false;
        let response = loop {
            let token = self.token(false).await?;
            let mut request = self.http.get(&url).bearer_auth(token);
            if existing > 0 {
                request = request.header("Range", format!("bytes={existing}-"));
            }
            let response = request.send().await?;
            if response.status().as_u16() == 401 && !refreshed {
                refreshed = true;
                if let Ok(email) = self.auth.default_email() {
                    self.auth.invalidate(&email);
                }
                continue;
            }
            break response;
        };
        let status = response.status();

        if status.as_u16() == 416 {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(AppError::Message("服务器拒绝了续传区间，请重试".into()));
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Api { status: status.as_u16(), message: api_message(&body) });
        }

        let append = status.as_u16() == 206 && existing > 0;
        if !append {
            existing = 0;
        }
        let total = if status.as_u16() == 206 {
            response
                .headers()
                .get("content-range")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.rsplit('/').next())
                .and_then(|value| value.parse::<u64>().ok())
                .or(file.size)
        } else {
            response.content_length().or(file.size)
        };

        let mut handle = if append {
            tokio::fs::OpenOptions::new().append(true).open(&part).await?
        } else {
            tokio::fs::File::create(&part).await?
        };
        let mut received = existing;
        progress(received, total);
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                handle.flush().await?;
                return Err(AppError::Cancelled);
            }
            let chunk = chunk?;
            handle.write_all(&chunk).await?;
            received += chunk.len() as u64;
            progress(received, total);
        }
        handle.flush().await?;
        drop(handle);

        if let Some(expected) = file.size {
            if export_mime.is_none() {
                let written = tokio::fs::metadata(&part).await?.len();
                if written != expected {
                    return Err(AppError::Message(format!(
                        "下载不完整：期望 {expected} 字节，实际 {written} 字节"
                    )));
                }
            }
        }
        tokio::fs::rename(&part, dest).await?;
        Ok(dest.to_path_buf())
    }

    /// Resumable upload in 8 MiB chunks. Every chunk is retried on transport
    /// or 5xx failures; a cancelled upload keeps its session so a retry can
    /// continue from the byte count Google already has.
    pub async fn upload_file(
        &self,
        local: &Path,
        parent: &str,
        name: &str,
        progress: &ProgressFn,
        cancel: &CancelToken,
    ) -> Result<FileMeta> {
        let total = tokio::fs::metadata(local).await?.len();
        let mime = mime_guess::from_path(local).first_or_octet_stream().to_string();
        let mut refreshed = false;
        let session = loop {
            let token = self.token(false).await?;
            let response = self
                .http
                .post(format!("{UPLOAD_API}/files?uploadType=resumable&supportsAllDrives=true&fields={FILE_FIELDS}"))
                .bearer_auth(&token)
                .header("X-Upload-Content-Type", &mime)
                .header("X-Upload-Content-Length", total.to_string())
                .json(&json!({ "name": name, "parents": [parent] }))
                .send()
                .await?;
            if response.status().as_u16() == 401 && !refreshed {
                refreshed = true;
                if let Ok(email) = self.auth.default_email() {
                    self.auth.invalidate(&email);
                }
                continue;
            }
            break response;
        };
        if !session.status().is_success() {
            let status = session.status().as_u16();
            let body = session.text().await.unwrap_or_default();
            return Err(AppError::Api { status, message: api_message(&body) });
        }
        let session_url = session
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Message("上传会话缺少 Location 响应头".into()))?
            .to_string();

        if total == 0 {
            let response = self
                .http
                .put(&session_url)
                .header("Content-Length", "0")
                .header("Content-Range", "bytes */0")
                .body(Vec::new())
                .send()
                .await?;
            return self.finish_upload(response, 0, total, progress).await;
        }

        let mut file = tokio::fs::File::open(local).await?;
        let mut buffer = vec![0u8; CHUNK];
        let mut sent: u64 = 0;
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = self.http.delete(&session_url).send().await;
                return Err(AppError::Cancelled);
            }
            let read = read_full(&mut file, &mut buffer).await?;
            if read == 0 {
                break;
            }
            let start = sent;
            let end = sent + read as u64 - 1;
            let body = buffer[..read].to_vec();
            let mut attempt = 0;
            loop {
                let response = self
                    .http
                    .put(&session_url)
                    .header("Content-Length", read.to_string())
                    .header("Content-Range", format!("bytes {start}-{end}/{total}"))
                    .body(body.clone())
                    .send()
                    .await;
                match response {
                    Ok(response) if response.status().as_u16() == 308 => break,
                    Ok(response) if response.status().is_success() => {
                        return self.finish_upload(response, end + 1, total, progress).await;
                    }
                    Ok(response) if response.status().is_server_error() && attempt < 3 => {
                        attempt += 1;
                        tokio::time::sleep(backoff(attempt)).await;
                    }
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let text = response.text().await.unwrap_or_default();
                        return Err(AppError::Api { status, message: api_message(&text) });
                    }
                    Err(error) if attempt < 3 => {
                        attempt += 1;
                        eprintln!("upload chunk retry: {error}");
                        tokio::time::sleep(backoff(attempt)).await;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            sent = end + 1;
            progress(sent, Some(total));
        }
        Err(AppError::Message("上传结束但服务器没有返回文件信息".into()))
    }

    async fn finish_upload(
        &self,
        response: reqwest::Response,
        sent: u64,
        total: u64,
        progress: &ProgressFn,
    ) -> Result<FileMeta> {
        progress(sent, Some(total));
        let value: Value = response.json().await?;
        Ok(serde_json::from_value::<RawFile>(value)?.into_meta())
    }

    /// First `limit` bytes of a text file, decoded lossily for preview.
    pub async fn text_preview(&self, file: &FileMeta, limit: u64) -> Result<(String, bool)> {
        let limit = limit.min(MAX_TEXT_PREVIEW);
        let mut refreshed = false;
        let response = loop {
            let token = self.token(false).await?;
            let response = self
                .http
                .get(format!("{API}/files/{}?alt=media&supportsAllDrives=true", encode(&file.id)))
                .bearer_auth(token)
                .header("Range", format!("bytes=0-{}", limit.saturating_sub(1)))
                .send()
                .await?;
            if response.status().as_u16() == 401 && !refreshed {
                refreshed = true;
                if let Ok(email) = self.auth.default_email() {
                    self.auth.invalidate(&email);
                }
                continue;
            }
            break response;
        };
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Api { status, message: api_message(&body) });
        }
        let bytes = response.bytes().await?;
        let truncated = file.size.map(|size| size > limit).unwrap_or(bytes.len() as u64 >= limit);
        Ok((String::from_utf8_lossy(&bytes).to_string(), truncated))
    }
}

fn api_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value["error"]["message"]
                .as_str()
                .map(str::to_string)
                .or_else(|| value["error_description"].as_str().map(str::to_string))
                .or_else(|| value["error"].as_str().map(str::to_string))
        })
        .unwrap_or_else(|| body.chars().take(400).collect())
}

fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(300 * 2u64.pow(attempt.saturating_sub(1)).min(8))
}

fn encode(value: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn escape_query(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

pub fn build_query(request: &ListRequest) -> String {
    let folder = request.folder_id.clone().unwrap_or_else(|| "root".into());
    let search = request.query.clone().unwrap_or_default();
    let search = search.trim();
    let name_filter = if search.is_empty() {
        String::new()
    } else {
        let escaped = escape_query(search);
        format!(" and (name contains '{escaped}' or fullText contains '{escaped}')")
    };

    match request.view.as_str() {
        "recent" => format!("trashed = false{name_filter}"),
        "starred" => format!("starred = true and trashed = false{name_filter}"),
        "trash" => "trashed = true".to_string(),
        "shared" => format!("sharedWithMe = true and trashed = false{name_filter}"),
        "search" if !search.is_empty() => format!("trashed = false{name_filter}"),
        "search" => "trashed = false".to_string(),
        _ => format!("'{folder}' in parents and trashed = false{name_filter}"),
    }
}

pub fn sort_files(files: &mut [FileMeta], sort_by: &str, sort_dir: &str) {
    let descending = sort_dir == "desc";
    files.sort_by(|left, right| {
        let left_folder = left.is_folder();
        let right_folder = right.is_folder();
        if left_folder != right_folder {
            return if left_folder { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
        }
        let ordering = match sort_by {
            "size" => left.size.unwrap_or(0).cmp(&right.size.unwrap_or(0)),
            "modified" => left.modified_time.cmp(&right.modified_time),
            "type" => left.mime_type.cmp(&right.mime_type),
            _ => natural_cmp(&left.name, &right.name),
        };
        if descending { ordering.reverse() } else { ordering }
    });
}

/// Case-insensitive natural ordering: `img2` sorts before `img10`.
pub fn natural_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let mut left_chars = left.chars().peekable();
    let mut right_chars = right.chars().peekable();
    loop {
        match (left_chars.peek().copied(), right_chars.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => {
                if a.is_ascii_digit() && b.is_ascii_digit() {
                    let mut left_number = String::new();
                    while let Some(c) = left_chars.peek().copied().filter(|c| c.is_ascii_digit()) {
                        left_number.push(c);
                        left_chars.next();
                    }
                    let mut right_number = String::new();
                    while let Some(c) = right_chars.peek().copied().filter(|c| c.is_ascii_digit()) {
                        right_number.push(c);
                        right_chars.next();
                    }
                    let left_value: u128 = left_number.trim_start_matches('0').parse().unwrap_or(0);
                    let right_value: u128 = right_number.trim_start_matches('0').parse().unwrap_or(0);
                    if left_value != right_value {
                        return left_value.cmp(&right_value);
                    }
                } else {
                    if a != b {
                        return a.cmp(&b);
                    }
                    left_chars.next();
                    right_chars.next();
                }
            }
        }
    }
}

/// `report.pdf` -> `report (2).pdf`, skipping names already taken.
pub fn disambiguate(name: &str, taken: &[String]) -> String {
    if !taken.iter().any(|existing| existing == name) {
        return name.to_string();
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or(name);
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 2..1000 {
        let candidate = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        if !taken.iter().any(|existing| existing == &candidate) {
            return candidate;
        }
    }
    format!("{stem} ({})", chrono::Utc::now().timestamp())
}

pub fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

async fn read_full(file: &mut tokio::fs::File, buffer: &mut [u8]) -> Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        let read = file.read(&mut buffer[filled..]).await?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, mime: &str, size: u64) -> FileMeta {
        FileMeta {
            id: name.into(),
            name: name.into(),
            mime_type: mime.into(),
            size: Some(size),
            modified_time: None,
            created_time: None,
            owner: None,
            owner_email: None,
            parents: vec![],
            starred: false,
            trashed: false,
            md5: None,
            web_view_link: None,
            thumbnail_link: None,
            icon_type: icon_type_for(mime).into(),
            shared: false,
            can_edit: true,
            can_download: true,
        }
    }

    #[test]
    fn folder_query_is_scoped_and_excludes_trash() {
        let request = ListRequest { view: "folder".into(), folder_id: Some("abc".into()), ..Default::default() };
        assert_eq!(build_query(&request), "'abc' in parents and trashed = false");
    }

    #[test]
    fn search_escapes_quotes_and_matches_full_text() {
        let request = ListRequest {
            view: "search".into(),
            query: Some("o'brien".into()),
            ..Default::default()
        };
        let query = build_query(&request);
        assert!(query.contains("name contains 'o\\'brien'"));
        assert!(query.contains("fullText contains 'o\\'brien'"));
        assert!(!query.contains(" in parents "));
    }

    #[test]
    fn sorting_keeps_folders_first_and_is_natural() {
        let mut files = vec![
            file("img10.png", "image/png", 10),
            file("img2.png", "image/png", 20),
            file("z", "application/vnd.google-apps.folder", 0),
        ];
        sort_files(&mut files, "name", "asc");
        assert_eq!(files[0].name, "z");
        assert_eq!(files[1].name, "img2.png");
        assert_eq!(files[2].name, "img10.png");
    }

    #[test]
    fn disambiguates_download_names() {
        let taken = vec!["a.pdf".to_string(), "a (2).pdf".to_string()];
        assert_eq!(disambiguate("a.pdf", &taken), "a (3).pdf");
        assert_eq!(disambiguate("b.pdf", &taken), "b.pdf");
        assert_eq!(disambiguate("README", &["README".into()]), "README (2)");
    }

    #[test]
    fn export_mapping_covers_google_native_types() {
        assert_eq!(export_target_for("application/vnd.google-apps.spreadsheet").unwrap().1, "xlsx");
        assert!(export_target_for("image/png").is_none());
        assert!(pdf_export_available("application/vnd.google-apps.document"));
    }

    #[test]
    fn icon_type_covers_the_interesting_mimes() {
        assert_eq!(icon_type_for("application/vnd.google-apps.folder"), "folder");
        assert_eq!(icon_type_for("application/vnd.google-apps.spreadsheet"), "sheet");
        assert_eq!(icon_type_for("video/mp4"), "video");
        assert_eq!(icon_type_for("application/pdf"), "pdf");
        assert_eq!(icon_type_for("application/zip"), "archive");
        assert_eq!(icon_type_for("application/octet-stream"), "file");
    }
}
