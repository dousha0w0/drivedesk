use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::error::{AppError, Result};
use crate::paths::AppPaths;
use crate::secrets::SecretStore;

const SCOPE: &str = "https://www.googleapis.com/auth/drive";
/// Optional OAuth client shipped with the build. A Desktop client's secret is
/// public by definition, so embedding the pair is normal for installed apps;
/// what it cannot do is bypass Google's verification rules (see README).
const BUILTIN_CLIENT_ID: Option<&str> = option_env!("DRIVEDESK_GOOGLE_CLIENT_ID");
const BUILTIN_CLIENT_SECRET: Option<&str> = option_env!("DRIVEDESK_GOOGLE_CLIENT_SECRET");
const BUILTIN_RUNTIME_ID: &str = "DRIVEDESK_GOOGLE_CLIENT_ID";
const BUILTIN_RUNTIME_SECRET: &str = "DRIVEDESK_GOOGLE_CLIENT_SECRET";
const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
    pub is_default: bool,
    pub has_client: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredAccount {
    email: String,
    name: String,
    picture: Option<String>,
    /// `builtin` or `imported`. Refresh must use the client that produced the
    /// token; older files without the field were imported clients.
    #[serde(default)]
    client: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct AccountsFile {
    #[serde(default)]
    default_email: Option<String>,
    #[serde(default)]
    accounts: Vec<StoredAccount>,
}

#[derive(Clone, Debug)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub token_uri: String,
}

#[derive(Deserialize)]
struct InstalledClient {
    client_id: String,
    #[serde(default)]
    client_secret: String,
    #[serde(default)]
    token_uri: Option<String>,
}

#[derive(Deserialize)]
struct ClientFile {
    #[serde(default)]
    installed: Option<InstalledClient>,
    #[serde(default)]
    web: Option<InstalledClient>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: i64,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct AboutUser {
    email_address: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    photo_link: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientSource {
    /// Imported client if present, else the built-in one.
    Auto,
    Builtin,
    #[allow(dead_code)]
    Imported,
}

pub struct AuthStore {
    paths: AppPaths,
    secrets: SecretStore,
    http: reqwest::Client,
    state: Mutex<AuthState>,
}

struct AuthState {
    file: AccountsFile,
    tokens: HashMap<String, (String, Instant)>,
    client: Option<OAuthClient>,
}

impl AuthStore {
    pub fn new(paths: AppPaths) -> Self {
        let secrets = SecretStore::new(paths.data_dir.clone());
        let file = std::fs::read_to_string(paths.accounts_file())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        let client = read_client(&paths.client_file()).ok();
        Self {
            paths,
            secrets,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client"),
            state: Mutex::new(AuthState { file, tokens: HashMap::new(), client }),
        }
    }

    /// OAuth client baked into the binary (runtime env wins, so a dev build can
    /// be pointed at a different project without recompiling).
    pub fn builtin_client() -> Option<OAuthClient> {
        let id = std::env::var(BUILTIN_RUNTIME_ID)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| BUILTIN_CLIENT_ID.map(str::to_string))?;
        let secret = std::env::var(BUILTIN_RUNTIME_SECRET)
            .ok()
            .or_else(|| BUILTIN_CLIENT_SECRET.map(str::to_string))
            .unwrap_or_default();
        Some(OAuthClient { client_id: id, client_secret: secret, token_uri: DEFAULT_TOKEN_URI.to_string() })
    }

    pub fn has_builtin_client() -> bool {
        Self::builtin_client().is_some()
    }

    pub fn has_any_client(&self) -> bool {
        self.paths.client_file().exists() || Self::has_builtin_client()
    }

    pub fn storage_label(&self) -> &'static str {
        self.secrets.kind().label()
    }

    pub fn accounts(&self) -> Vec<Account> {
        let state = self.state.lock().unwrap();
        let default = state.file.default_email.clone();
        let has_client = self.paths.client_file().exists() || Self::has_builtin_client();
        state
            .file
            .accounts
            .iter()
            .map(|account| Account {
                email: account.email.clone(),
                name: account.name.clone(),
                picture: account.picture.clone(),
                is_default: default.as_deref() == Some(account.email.as_str()),
                has_client,
            })
            .collect()
    }

    pub fn default_email(&self) -> Result<String> {
        let state = self.state.lock().unwrap();
        state
            .file
            .default_email
            .clone()
            .or_else(|| state.file.accounts.first().map(|account| account.email.clone()))
            .ok_or(AppError::NotSignedIn)
    }

    /// Drop the cached access token so the next call refreshes it.
    pub fn invalidate(&self, email: &str) {
        self.state.lock().unwrap().tokens.remove(email);
    }

    pub fn switch(&self, email: &str) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if !state.file.accounts.iter().any(|account| account.email == email) {
            return Err(AppError::Invalid(format!("unknown account: {email}")));
        }
        state.file.default_email = Some(email.to_string());
        self.persist(&state.file)
    }

    pub fn logout(&self, email: &str) -> Result<()> {
        self.secrets.delete(email)?;
        let mut state = self.state.lock().unwrap();
        state.file.accounts.retain(|account| account.email != email);
        state.tokens.remove(email);
        if state.file.default_email.as_deref() == Some(email) {
            state.file.default_email = state.file.accounts.first().map(|account| account.email.clone());
        }
        self.persist(&state.file)
    }

    /// Import (or reuse) the Desktop OAuth client and run the loopback flow.
    /// Import a client JSON if given, otherwise reuse a stored client and fall
    /// back to the one built into the binary.
    pub async fn login(&self, client_json: Option<PathBuf>) -> Result<Account> {
        self.login_with(client_json, ClientSource::Auto).await
    }

    /// One-click sign-in: always uses the built-in client, ignoring any
    /// previously imported one.
    pub async fn login_builtin(&self) -> Result<Account> {
        self.login_with(None, ClientSource::Builtin).await
    }

    async fn login_with(&self, client_json: Option<PathBuf>, source: ClientSource) -> Result<Account> {
        let mut kind = "imported";
        if let Some(path) = client_json {
            let parsed = read_client(&path)?;
            let raw = std::fs::read(&path)?;
            std::fs::write(self.paths.client_file(), raw)?;
            let mut state = self.state.lock().unwrap();
            state.client = Some(parsed);
            kind = "imported";
        }

        let client = match source {
            ClientSource::Builtin => {
                kind = "builtin";
                Self::builtin_client().ok_or_else(|| {
                    AppError::Message(
                        "此构建没有内置 Google OAuth 客户端，请改为导入你自己的桌面客户端 JSON。".into(),
                    )
                })?
            }
            ClientSource::Imported => {
                let stored = self.state.lock().unwrap().client.clone();
                stored.ok_or_else(|| AppError::Message("请先导入 Google OAuth 桌面客户端 JSON".into()))?
            }
            ClientSource::Auto => {
                let stored = self.state.lock().unwrap().client.clone();
                match (stored, Self::builtin_client()) {
                    (Some(client), _) => client,
                    (None, Some(client)) => {
                        kind = "builtin";
                        client
                    }
                    (None, None) => {
                        return Err(AppError::Message(
                            "请先导入 Google OAuth 桌面客户端 JSON".into(),
                        ))
                    }
                }
            }
        };

        let verifier: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let csrf: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect();

        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let redirect = format!("http://127.0.0.1:{port}/callback");

        let authorize_url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent%20select_account&code_challenge={}&code_challenge_method=S256&state={}",
            urlencode(&client.client_id),
            urlencode(&redirect),
            urlencode(SCOPE),
            urlencode(&challenge),
            urlencode(&csrf),
        );

        open_external(&authorize_url)?;

        let (code, returned_state) = wait_for_code(listener).await?;
        if returned_state.as_deref() != Some(csrf.as_str()) {
            return Err(AppError::Message("OAuth state mismatch".into()));
        }

        let token = self
            .exchange(
                &client,
                &[
                    ("code", code.as_str()),
                    ("client_id", client.client_id.as_str()),
                    ("client_secret", client.client_secret.as_str()),
                    ("redirect_uri", redirect.as_str()),
                    ("grant_type", "authorization_code"),
                    ("code_verifier", verifier.as_str()),
                ],
            )
            .await?;

        let refresh_token = token
            .refresh_token
            .clone()
            .ok_or_else(|| AppError::Message("Google 未返回 refresh_token".into()))?;

        let about = self.fetch_about(&token.access_token).await?;
        let email = about.email_address;
        let name = about.display_name.unwrap_or_else(|| email.clone());

        self.secrets.set(&email, &refresh_token)?;

        let account = Account {
            email: email.clone(),
            name: name.clone(),
            picture: about.photo_link,
            is_default: true,
            has_client: true,
        };

        {
            let mut state = self.state.lock().unwrap();
            state.tokens.insert(
                email.clone(),
                (token.access_token, Instant::now() + Duration::from_secs(token.expires_in.max(60) as u64)),
            );
            if let Some(existing) = state.file.accounts.iter_mut().find(|account| account.email == email) {
                existing.name = name;
                existing.picture = account.picture.clone();
                existing.client = Some(kind.to_string());
            } else {
                state.file.accounts.push(StoredAccount {
                    email: email.clone(),
                    name,
                    picture: account.picture.clone(),
                    client: Some(kind.to_string()),
                });
            }
            state.file.default_email = Some(email);
            self.persist(&state.file)?;
        }

        Ok(account)
    }

    /// Valid access token for an account, refreshed on demand.
    pub async fn access_token(&self, email: &str) -> Result<String> {
        {
            let state = self.state.lock().unwrap();
            if let Some((token, expires)) = state.tokens.get(email) {
                if *expires > Instant::now() + Duration::from_secs(30) {
                    return Ok(token.clone());
                }
            }
        }

        let refresh = self
            .secrets
            .get(email)?
            .ok_or(AppError::NotSignedIn)?;
        let kind = {
            let state = self.state.lock().unwrap();
            state
                .file
                .accounts
                .iter()
                .find(|account| account.email == email)
                .and_then(|account| account.client.clone())
                .unwrap_or_else(|| "imported".into())
        };
        let client = if kind == "builtin" {
            Self::builtin_client().ok_or_else(|| {
                AppError::Message("账号使用内置 OAuth 客户端，但当前构建没有内置客户端，请重新登录。".into())
            })?
        } else {
            let state = self.state.lock().unwrap();
            state.client.clone().ok_or(AppError::NotSignedIn)?
        };

        let token = self
            .exchange(
                &client,
                &[
                    ("client_id", client.client_id.as_str()),
                    ("client_secret", client.client_secret.as_str()),
                    ("refresh_token", refresh.as_str()),
                    ("grant_type", "refresh_token"),
                ],
            )
            .await
            .map_err(|error| match error {
                AppError::Message(message) if message.contains("invalid_grant") => AppError::NotSignedIn,
                other => other,
            })?;

        let mut state = self.state.lock().unwrap();
        state.tokens.insert(
            email.to_string(),
            (token.access_token.clone(), Instant::now() + Duration::from_secs(token.expires_in.max(60) as u64)),
        );
        Ok(token.access_token)
    }

    async fn exchange(&self, client: &OAuthClient, form: &[(&str, &str)]) -> Result<TokenResponse> {
        let response = self.http.post(&client.token_uri).form(form).send().await?;
        let status = response.status();
        let parsed: TokenResponse = response.json().await?;
        if !status.is_success() || parsed.error.is_some() {
            let message = parsed
                .error_description
                .or(parsed.error)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            return Err(AppError::Message(format!("token endpoint: {message}")));
        }
        Ok(parsed)
    }

    async fn fetch_about(&self, access_token: &str) -> Result<AboutUser> {
        let response = self
            .http
            .get("https://www.googleapis.com/drive/v3/about?fields=user(emailAddress,displayName,photoLink)")
            .bearer_auth(access_token)
            .send()
            .await?;
        let status = response.status();
        let body: serde_json::Value = response.json().await?;
        if !status.is_success() {
            return Err(AppError::Api {
                status: status.as_u16(),
                message: body.to_string(),
            });
        }
        let user: AboutUser = serde_json::from_value(body["user"].clone())?;
        Ok(user)
    }

    fn persist(&self, file: &AccountsFile) -> Result<()> {
        let raw = serde_json::to_string_pretty(file)?;
        std::fs::write(self.paths.accounts_file(), raw)?;
        Ok(())
    }
}

fn read_client(path: &Path) -> Result<OAuthClient> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| AppError::Message(format!("无法读取 OAuth 客户端 JSON: {error}")))?;
    let parsed: ClientFile = serde_json::from_str(&raw)
        .map_err(|error| AppError::Message(format!("OAuth 客户端 JSON 格式不正确: {error}")))?;
    // A `web` client cannot use a loopback redirect; Google rejects it at the
    // consent step, which is much harder to diagnose than saying so up front.
    if parsed.installed.is_none() && parsed.web.is_some() {
        return Err(AppError::Message(
            "这是「Web 应用」类型的 OAuth 客户端。请在 Google Cloud 重新创建「桌面应用（Desktop app）」类型并下载 JSON。".into(),
        ));
    }
    let installed = parsed
        .installed
        .ok_or_else(|| AppError::Message("OAuth 客户端 JSON 缺少 installed 段".into()))?;
    if installed.client_id.trim().is_empty() {
        return Err(AppError::Message("OAuth 客户端缺少 client_id".into()));
    }
    Ok(OAuthClient {
        client_id: installed.client_id,
        client_secret: installed.client_secret,
        token_uri: installed.token_uri.unwrap_or_else(|| DEFAULT_TOKEN_URI.to_string()),
    })
}

fn urlencode(value: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

pub fn open_external(target: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");

    command
        .arg(target)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| AppError::Message(format!("无法打开系统浏览器: {error}")))
}

/// Accept loopback redirects until one carries `code`/`error` or the login times
/// out. Browsers commonly open a second connection for `/favicon.ico`; those are
/// answered and ignored instead of failing the flow.
async fn wait_for_code(listener: TcpListener) -> Result<(String, Option<String>)> {
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(AppError::Message("登录超时，请重试".into()));
        }
        let (mut socket, _) = tokio::time::timeout(remaining, listener.accept())
            .await
            .map_err(|_| AppError::Message("登录超时，请重试".into()))??;
        let mut buffer = vec![0u8; 8192];
        let read = tokio::time::timeout(Duration::from_secs(10), socket.read(&mut buffer))
            .await
            .map_err(|_| AppError::Message("读取 OAuth 回调超时".into()))??;
        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or_default()
            .to_string();

        let parsed = url::Url::parse(&format!("http://127.0.0.1{target}")).ok();
        let params: HashMap<String, String> = parsed
            .map(|url| url.query_pairs().into_owned().collect())
            .unwrap_or_default();

        if let Some(error) = params.get("error") {
            let body = page("授权被拒绝", &format!("Google 返回：{error}"));
            let _ = respond(&mut socket, &body).await;
            return Err(AppError::Message(format!("授权被拒绝：{error}")));
        }

        match params.get("code") {
            Some(code) => {
                let body = page("DriveDesk 已获得授权", "可以关闭这个页面，回到 DriveDesk 继续使用。");
                let _ = respond(&mut socket, &body).await;
                return Ok((code.clone(), params.get("state").cloned()));
            }
            None => {
                let _ = respond(&mut socket, "<html><body></body></html>").await;
            }
        }
    }
}

async fn respond(socket: &mut tokio::net::TcpStream, body: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.as_bytes().len(),
        body
    );
    socket.write_all(response.as_bytes()).await?;
    socket.flush().await?;
    Ok(())
}

fn page(title: &str, detail: &str) -> String {
    format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{title}</title>\
         <body style=\"font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;display:flex;height:100vh;margin:0;align-items:center;justify-content:center;background:#fafafa;color:#111\">\
         <div style=\"max-width:420px;text-align:center\"><h1 style=\"font-size:18px;font-weight:600\">{title}</h1>\
         <p style=\"font-size:13px;color:#555;line-height:1.7\">{detail}</p></div></body>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_client_json_without_installed_block() {
        let dir = std::env::temp_dir().join(format!("drivedesk-client-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("client.json");
        std::fs::write(&file, r#"{"client_id":"x"}"#).unwrap();
        assert!(read_client(&file).is_err());

        std::fs::write(&file, r#"{"web":{"client_id":"id.apps.googleusercontent.com","client_secret":"secret"}}"#).unwrap();
        let web_error = read_client(&file).unwrap_err().to_string();
        assert!(web_error.contains("桌面应用"));

        std::fs::write(
            &file,
            r#"{"installed":{"client_id":"id.apps.googleusercontent.com","client_secret":"secret","token_uri":"https://oauth2.googleapis.com/token"}}"#,
        )
        .unwrap();
        let client = read_client(&file).unwrap();
        assert_eq!(client.client_id, "id.apps.googleusercontent.com");
        assert_eq!(client.token_uri, "https://oauth2.googleapis.com/token");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pkce_challenge_is_url_safe_and_stable() {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(b"verifier"));
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
        assert_eq!(challenge, URL_SAFE_NO_PAD.encode(Sha256::digest(b"verifier")));
    }
}
