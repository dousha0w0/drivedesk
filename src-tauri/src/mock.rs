use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::auth::Account;
use crate::drive::{disambiguate, sort_files, with_suffix, CancelToken, ProgressFn};
use crate::error::{AppError, Result};
use crate::types::*;

/// In-memory Drive used by `DRIVEDESK_MOCK=1`.
///
/// It exists so the whole application — table, preview, transfers, smoke
/// harness — can be exercised without Google credentials. Downloads write real
/// bytes to real files and uploads read the real local file, so transfer code
/// paths are not stubbed out; only the remote side is synthetic.
pub struct MockDrive {
    state: Mutex<State>,
    upload_dir: PathBuf,
}

struct State {
    entries: Vec<Entry>,
    sequence: u64,
}

#[derive(Clone)]
struct Entry {
    meta: FileMeta,
    content: Content,
}

#[derive(Clone)]
enum Content {
    Text(String),
    Svg(String),
    Pdf,
    Bytes(u64),
    Local(PathBuf),
    None,
}

impl Entry {
    fn folder(id: &str, name: &str, parents: &[&str]) -> Self {
        Self {
            meta: FileMeta {
                id: id.into(),
                name: name.into(),
                mime_type: "application/vnd.google-apps.folder".into(),
                size: None,
                modified_time: Some("2026-09-01T09:00:00.000Z".into()),
                created_time: Some("2026-01-05T09:00:00.000Z".into()),
                owner: Some("Demo Account".into()),
                owner_email: Some("demo@drivedesk.local".into()),
                parents: parents.iter().map(|value| value.to_string()).collect(),
                starred: false,
                trashed: false,
                md5: None,
                web_view_link: None,
                thumbnail_link: None,
                icon_type: "folder".into(),
                shared: false,
                can_edit: true,
                can_download: true,
            },
            content: Content::None,
        }
    }

    fn file(id: &str, name: &str, mime: &str, parents: &[&str], content: Content, modified: &str, size: Option<u64>) -> Self {
        Self {
            meta: FileMeta {
                id: id.into(),
                name: name.into(),
                mime_type: mime.into(),
                size,
                modified_time: Some(modified.into()),
                created_time: Some("2026-02-11T09:00:00.000Z".into()),
                owner: Some("Demo Account".into()),
                owner_email: Some("demo@drivedesk.local".into()),
                parents: parents.iter().map(|value| value.to_string()).collect(),
                starred: false,
                trashed: false,
                md5: None,
                web_view_link: Some(format!("https://drive.google.com/file/d/{id}/view")),
                thumbnail_link: None,
                icon_type: icon_type_for(mime).into(),
                shared: false,
                can_edit: true,
                can_download: true,
            },
            content,
        }
    }
}

impl MockDrive {
    pub fn new(upload_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&upload_dir).ok();
        Self { state: Mutex::new(State { entries: seed(), sequence: 0 }), upload_dir }
    }

    fn next_id(&self, state: &mut State) -> String {
        state.sequence += 1;
        format!("mock-{}-{}", state.sequence, chrono::Utc::now().timestamp_millis())
    }

    pub fn accounts(&self) -> Vec<Account> {
        vec![Account {
            email: "demo@drivedesk.local".into(),
            name: "Demo Account".into(),
            picture: None,
            is_default: true,
            has_client: true,
        }]
    }

    pub fn quota(&self) -> Quota {
        let state = self.state.lock().unwrap();
        let usage: u64 = state
            .entries
            .iter()
            .filter_map(|entry| entry.meta.size)
            .sum::<u64>()
            + 3_400_000_000;
        Quota {
            limit: Some(15 * 1024 * 1024 * 1024),
            usage: Some(usage),
            drive_usage: Some(usage),
            user: Some("demo@drivedesk.local".into()),
        }
    }

    pub fn list(&self, request: &ListRequest) -> Result<ListResponse> {
        let state = self.state.lock().unwrap();
        let query = request.query.clone().unwrap_or_default().to_lowercase();
        let folder = request.folder_id.clone().unwrap_or_else(|| "root".into());

        let mut files: Vec<FileMeta> = state
            .entries
            .iter()
            .filter(|entry| entry.meta.id != "root")
            .filter(|entry| match request.view.as_str() {
                "recent" => !entry.meta.trashed,
                "starred" => entry.meta.starred && !entry.meta.trashed,
                "trash" => entry.meta.trashed,
                "shared" => entry.meta.shared && !entry.meta.trashed,
                "search" => !entry.meta.trashed,
                _ => entry.meta.parents.iter().any(|parent| parent == &folder) && !entry.meta.trashed,
            })
            .filter(|entry| {
                if query.is_empty() {
                    return true;
                }
                let name = entry.meta.name.to_lowercase();
                if name.contains(&query) {
                    return true;
                }
                match &entry.content {
                    Content::Text(text) | Content::Svg(text) => text.to_lowercase().contains(&query),
                    _ => false,
                }
            })
            .map(|entry| entry.meta.clone())
            .collect();

        if request.view == "recent" {
            files.sort_by(|left, right| right.modified_time.cmp(&left.modified_time));
        } else {
            sort_files(
                &mut files,
                request.sort_by.as_deref().unwrap_or("name"),
                request.sort_dir.as_deref().unwrap_or("asc"),
            );
        }

        let page_size = request.page_size.unwrap_or(200) as usize;
        let offset: usize = request
            .page_token
            .as_deref()
            .and_then(|token| token.parse().ok())
            .unwrap_or(0);
        let slice: Vec<FileMeta> = files.iter().skip(offset).take(page_size).cloned().collect();
        let next = if offset + page_size < files.len() {
            Some((offset + page_size).to_string())
        } else {
            None
        };
        Ok(ListResponse { files: slice, next_page_token: next, total: Some(files.len() as u64) })
    }

    pub fn metadata(&self, id: &str) -> Result<FileMeta> {
        self.entry(id).map(|entry| entry.meta)
    }

    pub fn path(&self, id: &str) -> Result<Vec<Crumb>> {
        let mut crumbs = Vec::new();
        let mut cursor = id.to_string();
        for _ in 0..64 {
            let entry = self.entry(&cursor)?;
            crumbs.push(Crumb { id: entry.meta.id.clone(), name: entry.meta.name.clone() });
            match entry.meta.parents.first() {
                Some(parent) => cursor = parent.clone(),
                None => break,
            }
        }
        crumbs.reverse();
        Ok(crumbs)
    }

    pub fn create_folder(&self, parent: &str, name: &str) -> Result<FileMeta> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Invalid("名称不能为空".into()));
        }
        let mut state = self.state.lock().unwrap();
        self.ensure_folder(&state, parent)?;
        let id = self.next_id(&mut state);
        let mut entry = Entry::folder(&id, name, &[parent]);
        entry.meta.modified_time = Some(now_iso());
        entry.meta.created_time = entry.meta.modified_time.clone();
        let meta = entry.meta.clone();
        state.entries.push(entry);
        Ok(meta)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<FileMeta> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Invalid("名称不能为空".into()));
        }
        let mut state = self.state.lock().unwrap();
        let entry = state
            .entries
            .iter_mut()
            .find(|entry| entry.meta.id == id)
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))?;
        entry.meta.name = name.to_string();
        entry.meta.modified_time = Some(now_iso());
        Ok(entry.meta.clone())
    }

    pub fn move_file(&self, id: &str, parent: &str) -> Result<FileMeta> {
        let mut state = self.state.lock().unwrap();
        self.ensure_folder(&state, parent)?;
        let entry = state
            .entries
            .iter_mut()
            .find(|entry| entry.meta.id == id)
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))?;
        entry.meta.parents = vec![parent.to_string()];
        entry.meta.modified_time = Some(now_iso());
        Ok(entry.meta.clone())
    }

    pub fn copy(&self, id: &str, parent: &str) -> Result<FileMeta> {
        let mut state = self.state.lock().unwrap();
        self.ensure_folder(&state, parent)?;
        let source = state
            .entries
            .iter()
            .find(|entry| entry.meta.id == id)
            .cloned()
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))?;
        let new_id = self.next_id(&mut state);
        let mut clone = source;
        clone.meta.id = new_id;
        clone.meta.name = format!("{} 的副本", clone.meta.name);
        clone.meta.parents = vec![parent.to_string()];
        clone.meta.modified_time = Some(now_iso());
        clone.meta.web_view_link = None;
        let meta = clone.meta.clone();
        state.entries.push(clone);
        Ok(meta)
    }

    pub fn trash(&self, id: &str, trashed: bool) -> Result<FileMeta> {
        let mut state = self.state.lock().unwrap();
        let entry = state
            .entries
            .iter_mut()
            .find(|entry| entry.meta.id == id)
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))?;
        entry.meta.trashed = trashed;
        entry.meta.modified_time = Some(now_iso());
        Ok(entry.meta.clone())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let before = state.entries.len();
        state.entries.retain(|entry| entry.meta.id != id);
        if state.entries.len() == before {
            return Err(AppError::Invalid(format!("unknown file: {id}")));
        }
        Ok(())
    }

    pub fn star(&self, id: &str, starred: bool) -> Result<FileMeta> {
        let mut state = self.state.lock().unwrap();
        let entry = state
            .entries
            .iter_mut()
            .find(|entry| entry.meta.id == id)
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))?;
        entry.meta.starred = starred;
        Ok(entry.meta.clone())
    }

    pub fn unique_name(&self, parent: &str, name: &str) -> Result<String> {
        let state = self.state.lock().unwrap();
        let taken: Vec<String> = state
            .entries
            .iter()
            .filter(|entry| entry.meta.parents.iter().any(|value| value == parent))
            .map(|entry| entry.meta.name.clone())
            .collect();
        Ok(disambiguate(name, &taken))
    }

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
        let total = self.content_size(&file.id, export_mime)?;
        let mut handle = tokio::fs::File::create(&part).await?;
        let mut received: u64 = 0;
        progress(0, total);

        if let Some(source) = self.local_path(&file.id) {
            let mut reader = tokio::fs::File::open(source).await?;
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                if cancel.load(Ordering::Relaxed) {
                    handle.flush().await?;
                    return Err(AppError::Cancelled);
                }
                let read = reader.read(&mut buffer).await?;
                if read == 0 {
                    break;
                }
                handle.write_all(&buffer[..read]).await?;
                received += read as u64;
                progress(received, total);
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        } else if let Some(bytes) = self.small_payload(&file.id, export_mime)? {
            let total = bytes.len() as u64;
            let mut offset = 0;
            while offset < bytes.len() {
                if cancel.load(Ordering::Relaxed) {
                    handle.flush().await?;
                    return Err(AppError::Cancelled);
                }
                let end = (offset + 64 * 1024).min(bytes.len());
                handle.write_all(&bytes[offset..end]).await?;
                received += (end - offset) as u64;
                progress(received, Some(total));
                offset = end;
                tokio::time::sleep(Duration::from_millis(6)).await;
            }
        } else {
            let chunk = vec![0x5au8; 64 * 1024];
            let mut remaining = total.unwrap_or(0);
            while remaining > 0 {
                if cancel.load(Ordering::Relaxed) {
                    handle.flush().await?;
                    return Err(AppError::Cancelled);
                }
                let write = remaining.min(chunk.len() as u64) as usize;
                handle.write_all(&chunk[..write]).await?;
                remaining -= write as u64;
                received += write as u64;
                progress(received, total);
                tokio::time::sleep(Duration::from_millis(4)).await;
            }
        }

        handle.flush().await?;
        drop(handle);
        tokio::fs::rename(&part, dest).await?;
        Ok(dest.to_path_buf())
    }

    pub async fn upload_file(
        &self,
        local: &Path,
        parent: &str,
        name: &str,
        progress: &ProgressFn,
        cancel: &CancelToken,
    ) -> Result<FileMeta> {
        let size = tokio::fs::metadata(local).await?.len();
        {
            let state = self.state.lock().unwrap();
            self.ensure_folder(&state, parent)?;
        }
        let mut copy_target = self.upload_dir.join(format!(
            "{}-{}",
            chrono::Utc::now().timestamp_millis(),
            name
        ));
        if let Some(existing) = copy_target.file_name() {
            copy_target.set_file_name(format!("{}-{}", chrono::Utc::now().timestamp_millis(), existing.to_string_lossy()));
        }
        tokio::fs::copy(local, &copy_target).await?;

        progress(size, Some(size));
        if cancel.load(Ordering::Relaxed) {
            let _ = tokio::fs::remove_file(&copy_target).await;
            return Err(AppError::Cancelled);
        }

        let mime = mime_guess::from_path(local).first_or_octet_stream().to_string();
        let mut state = self.state.lock().unwrap();
        let id = self.next_id(&mut state);
        let mut entry = Entry::file(&id, name, &mime, &[parent], Content::Local(copy_target), &now_iso(), Some(size));
        entry.meta.owner = Some("Demo Account".into());
        let meta = entry.meta.clone();
        state.entries.push(entry);
        Ok(meta)
    }

    pub async fn text_preview(&self, file: &FileMeta, limit: u64) -> Result<(String, bool)> {
        let content = {
            let state = self.state.lock().unwrap();
            state
                .entries
                .iter()
                .find(|entry| entry.meta.id == file.id)
                .map(|entry| entry.content.clone())
                .unwrap_or(Content::None)
        };
        match content {
            Content::Text(text) | Content::Svg(text) => {
                let truncated = text.len() as u64 > limit;
                let mut slice = text;
                if truncated {
                    slice.truncate(limit as usize);
                }
                Ok((slice, truncated))
            }
            Content::Local(path) => {
                let bytes = tokio::fs::read(path).await?;
                let truncated = bytes.len() as u64 > limit;
                let take = (bytes.len() as u64).min(limit) as usize;
                Ok((String::from_utf8_lossy(&bytes[..take]).to_string(), truncated))
            }
            _ => Ok((String::new(), false)),
        }
    }

    fn local_path(&self, id: &str) -> Option<PathBuf> {
        let state = self.state.lock().unwrap();
        match state.entries.iter().find(|entry| entry.meta.id == id).map(|entry| entry.content.clone()) {
            Some(Content::Local(path)) => Some(path),
            _ => None,
        }
    }

    fn content_size(&self, id: &str, export_mime: Option<&str>) -> Result<Option<u64>> {
        if export_mime.is_some() {
            return Ok(Some(pdf_bytes("DriveDesk export").len() as u64));
        }
        let state = self.state.lock().unwrap();
        let entry = state
            .entries
            .iter()
            .find(|entry| entry.meta.id == id)
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))?;
        Ok(match &entry.content {
            Content::Text(text) | Content::Svg(text) => Some(text.len() as u64),
            Content::Pdf => Some(pdf_bytes("DriveDesk preview").len() as u64),
            Content::Bytes(size) => Some(*size),
            Content::Local(path) => std::fs::metadata(path).ok().map(|meta| meta.len()),
            Content::None => Some(0),
        })
    }

    /// Payloads that are cheap to materialize. Large synthetic files stream
    /// instead so a 24 MB download does not allocate 24 MB up front.
    fn small_payload(&self, id: &str, export_mime: Option<&str>) -> Result<Option<Vec<u8>>> {
        if export_mime.is_some() {
            return Ok(Some(pdf_bytes("DriveDesk export")));
        }
        let state = self.state.lock().unwrap();
        let entry = state
            .entries
            .iter()
            .find(|entry| entry.meta.id == id)
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))?;
        Ok(match &entry.content {
            Content::Text(text) | Content::Svg(text) => Some(text.clone().into_bytes()),
            Content::Pdf => Some(pdf_bytes("DriveDesk preview")),
            Content::Bytes(size) if *size <= 4 * 1024 * 1024 => Some(vec![0x5a; *size as usize]),
            Content::Local(path) if entry.meta.size.unwrap_or(0) <= 4 * 1024 * 1024 => std::fs::read(path).ok(),
            _ => None,
        })
    }

    fn entry(&self, id: &str) -> Result<Entry> {
        self.state
            .lock()
            .unwrap()
            .entries
            .iter()
            .find(|entry| entry.meta.id == id)
            .cloned()
            .ok_or_else(|| AppError::Invalid(format!("unknown file: {id}")))
    }

    fn ensure_folder(&self, state: &State, id: &str) -> Result<()> {
        state
            .entries
            .iter()
            .find(|entry| entry.meta.id == id && entry.meta.is_folder())
            .map(|_| ())
            .ok_or_else(|| AppError::Invalid(format!("not a folder: {id}")))
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn svg(label: &str, from: &str, to: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"720\" height=\"480\" viewBox=\"0 0 720 480\">\
           <defs><linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">\
             <stop offset=\"0\" stop-color=\"{from}\"/><stop offset=\"1\" stop-color=\"{to}\"/></linearGradient></defs>\
           <rect width=\"720\" height=\"480\" fill=\"url(#g)\"/>\
           <circle cx=\"540\" cy=\"140\" r=\"90\" fill=\"#ffffff\" opacity=\"0.18\"/>\
           <circle cx=\"180\" cy=\"330\" r=\"130\" fill=\"#ffffff\" opacity=\"0.12\"/>\
           <text x=\"48\" y=\"420\" font-family=\"Helvetica,Arial,sans-serif\" font-size=\"34\" fill=\"#ffffff\" opacity=\"0.92\">{label}</text>\
         </svg>"
    )
}

fn seed() -> Vec<Entry> {
    let mut entries = vec![Entry::folder("root", "我的云盘", &[])];
    entries.push(Entry::folder("projects", "项目", &["root"]));
    entries.push(Entry::folder("projects-2026", "2026 Q4", &["projects"]));
    entries.push(Entry::folder("projects-archive", "归档", &["projects"]));
    entries.push(Entry::folder("design", "设计", &["root"]));
    entries.push(Entry::folder("photos", "照片", &["root"]));
    entries.push(Entry::folder("docs", "文档", &["root"]));

    entries.push(Entry::file(
        "readme",
        "README.md",
        "text/markdown",
        &["root"],
        Content::Text(
            "# DriveDesk\n\n这是演示模式的数据，不需要 Google 账号。\n\n- 浏览文件夹、搜索、排序、右键菜单\n- 双击预览文本、图片、PDF\n- 选中文件后下载，进度显示在底部传输坞\n- 上传会读取真实本地文件\n\n把 `DRIVEDESK_MOCK=1` 去掉，导入自己的 Google OAuth 桌面客户端后即可连接真实云盘。\n"
                .into(),
        ),
        "2026-10-20T11:20:00.000Z",
        None,
    ));
    entries.push(Entry::file(
        "budget",
        "预算.csv",
        "text/csv",
        &["root"],
        Content::Text("月份,预算,实际\n7,12000,11840\n8,12000,13110\n9,15000,14220\n10,15000,15080\n".into()),
        "2026-10-18T08:12:00.000Z",
        None,
    ));
    entries.push(Entry::file(
        "demo-video",
        "产品演示.mp4",
        "video/mp4",
        &["root"],
        Content::Bytes(48 * 1024 * 1024),
        "2026-10-12T14:40:00.000Z",
        Some(48 * 1024 * 1024),
    ));
    entries.push(Entry::file(
        "installer",
        "安装包.zip",
        "application/zip",
        &["root"],
        Content::Bytes(24 * 1024 * 1024 + 913_000),
        "2026-09-30T03:10:00.000Z",
        Some(24 * 1024 * 1024 + 913_000),
    ));

    entries.push(Entry::file(
        "review-deck",
        "设计评审.docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &["projects-2026"],
        Content::Bytes(2 * 1024 * 1024 + 71_000),
        "2026-10-21T07:05:00.000Z",
        Some(2 * 1024 * 1024 + 71_000),
    ));
    entries.push(Entry::file(
        "schedule",
        "排期.xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        &["projects-2026"],
        Content::Bytes(512 * 1024),
        "2026-10-19T05:30:00.000Z",
        Some(512 * 1024),
    ));
    entries.push(Entry::file(
        "meeting",
        "会议纪要.txt",
        "text/plain",
        &["projects-2026"],
        Content::Text(
            "2026-10-20 评审会议\n\n1. 文件列表默认按名称自然排序，文件夹优先。\n2. 预览面板支持文本 / 图片 / PDF。\n3. 上传走 resumable session，大文件可续传。\n4. 下载完成后与 Drive 记录的 MD5 比对。\n"
                .into(),
        ),
        "2026-10-20T10:00:00.000Z",
        None,
    ));
    entries.push(Entry::file(
        "old-notes",
        "旧笔记.txt",
        "text/plain",
        &["projects-archive"],
        Content::Text("这是一份归档笔记，用来验证目录层级。\n".into()),
        "2025-12-02T02:00:00.000Z",
        None,
    ));

    entries.push(Entry::file(
        "cover",
        "封面.svg",
        "image/svg+xml",
        &["design"],
        Content::Svg(svg("Cover / 2026", "#6366f1", "#8b5cf6")),
        "2026-10-08T06:00:00.000Z",
        None,
    ));
    entries.push(Entry::file(
        "logo-dark",
        "Logo dark.svg",
        "image/svg+xml",
        &["design"],
        Content::Svg(svg("Logo dark", "#0f172a", "#334155")),
        "2026-09-27T06:00:00.000Z",
        None,
    ));
    entries.push(Entry::file(
        "spec",
        "交互规范.pdf",
        "application/pdf",
        &["design"],
        Content::Pdf,
        "2026-09-25T06:00:00.000Z",
        None,
    ));

    let palettes = [
        ("#f97316", "#ef4444"),
        ("#0ea5e9", "#6366f1"),
        ("#10b981", "#14b8a6"),
        ("#a855f7", "#ec4899"),
        ("#eab308", "#f59e0b"),
        ("#64748b", "#0f172a"),
    ];
    for index in 1..=40u32 {
        let (from, to) = palettes[index as usize % palettes.len()];
        let content = svg(&format!("IMG_{index:04}"), from, to);
        let size = content.len() as u64;
        let mime = "image/svg+xml".to_string();
        let name = format!("IMG_{index:04}.svg");
        let id = format!("photo-{index:04}");
        let day = (index % 27) + 1;
        entries.push(Entry::file(
            &id,
            &name,
            &mime,
            &["photos"],
            Content::Svg(content),
            &format!("2026-08-{day:02}T09:00:00.000Z"),
            Some(size),
        ));
    }

    entries.push(Entry::file(
        "reading-list",
        "阅读清单.md",
        "text/markdown",
        &["docs"],
        Content::Text(
            "# 阅读清单\n\n- [x] Designing Data-Intensive Applications\n- [ ] The Pragmatic Programmer\n- [ ] Rust for Rustaceans\n\n> 演示模式会把这里的文本直接渲染在预览面板。\n"
                .into(),
        ),
        "2026-10-16T09:00:00.000Z",
        None,
    ));
    entries.push(Entry::file(
        "empty",
        "空白文件.txt",
        "text/plain",
        &["docs"],
        Content::Text(String::new()),
        "2026-10-02T09:00:00.000Z",
        Some(0),
    ));
    entries.push(Entry::file(
        "contract",
        "合同.pdf",
        "application/pdf",
        &["docs"],
        Content::Pdf,
        "2026-08-14T09:00:00.000Z",
        None,
    ));
    entries.push(Entry::file(
        "old-build",
        "旧版本.zip",
        "application/zip",
        &["root"],
        Content::Bytes(8 * 1024 * 1024),
        "2026-03-03T09:00:00.000Z",
        Some(8 * 1024 * 1024),
    ));
    if let Some(entry) = entries.last_mut() {
        entry.meta.trashed = true;
    }
    entries
        .iter_mut()
        .find(|entry| entry.meta.id == "cover")
        .map(|entry| entry.meta.starred = true);

    for entry in &mut entries {
        if entry.meta.size.is_none() {
            if let Content::Text(text) | Content::Svg(text) = &entry.content {
                entry.meta.size = Some(text.len() as u64);
            }
        }
    }
    entries
}

/// Minimal, spec-valid PDF with a correct xref table, so the WebView's PDF
/// viewer can be exercised without shipping a fixture binary.
pub fn pdf_bytes(title: &str) -> Vec<u8> {
    let escaped = title.replace('\\', "").replace('(', "").replace(')', "");
    let stream = format!("BT /F1 20 Tf 56 780 Td ({escaped}) Tj ET");
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
    ];

    let mut body = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(body.len());
        body.push_str(&format!("{} 0 obj\n{}\nendobj\n", index + 1, object));
    }
    let xref_at = body.len();
    body.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for offset in &offsets {
        body.push_str(&format!("{offset:010} 00000 n \n"));
    }
    body.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        xref_at
    ));
    body.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_has_enough_rows_and_a_trashed_item() {
        let drive = MockDrive::new(std::env::temp_dir().join("drivedesk-mock-test"));
        let root = drive.list(&ListRequest { view: "folder".into(), folder_id: Some("root".into()), ..Default::default() }).unwrap();
        assert!(root.files.len() >= 5);
        let photos = drive.list(&ListRequest { view: "folder".into(), folder_id: Some("photos".into()), ..Default::default() }).unwrap();
        assert_eq!(photos.files.len(), 40);
        let trash = drive.list(&ListRequest { view: "trash".into(), ..Default::default() }).unwrap();
        assert_eq!(trash.files.len(), 1);
    }

    #[test]
    fn search_matches_names_and_text_content() {
        let drive = MockDrive::new(std::env::temp_dir().join("drivedesk-mock-test"));
        let by_name = drive.list(&ListRequest { view: "search".into(), query: Some("IMG_0007".into()), ..Default::default() }).unwrap();
        assert_eq!(by_name.files.len(), 1);
        let by_text = drive.list(&ListRequest {
            view: "search".into(),
            query: Some("resumable".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(by_text.files.len(), 1);
        assert_eq!(by_text.files[0].name, "会议纪要.txt");
    }

    #[test]
    fn mutations_are_visible_immediately() {
        let drive = MockDrive::new(std::env::temp_dir().join("drivedesk-mock-test"));
        let folder = drive.create_folder("root", "测试目录").unwrap();
        assert_eq!(folder.icon_type, "folder");
        let renamed = drive.rename(&folder.id, "改名后").unwrap();
        assert_eq!(renamed.name, "改名后");
        let copied = drive.copy("readme", "root").unwrap();
        assert!(copied.name.contains("副本"));
        let trashed = drive.trash(&copied.id, true).unwrap();
        assert!(trashed.trashed);
        assert_eq!(drive.path(&folder.id).unwrap().len(), 2);
    }

    #[test]
    fn unique_names_skip_taken_candidates() {
        let drive = MockDrive::new(std::env::temp_dir().join("drivedesk-mock-test"));
        assert_eq!(drive.unique_name("root", "README.md").unwrap(), "README (2).md");
    }

    #[test]
    fn generated_pdf_has_a_cross_reference_table() {
        let bytes = pdf_bytes("Preview");
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("xref"));
        assert!(text.trim_end().ends_with("%%EOF"));
        assert!(text.contains("/MediaBox [0 0 595 842]"));
    }
}
