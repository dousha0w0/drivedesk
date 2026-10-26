use serde::{Serialize, Serializer};

/// Errors crossing the IPC boundary. Everything is flattened to a message the
/// UI can show verbatim; the frontend never pattern-matches on the variant.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("not signed in")]
    NotSignedIn,
    #[error("Google API error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("file error: {0}")]
    Io(String),
    #[error("cancelled")]
    Cancelled,
    #[error("invalid request: {0}")]
    Invalid(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(error.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            AppError::Network("request timed out".into())
        } else {
            AppError::Network(error.to_string())
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Message(format!("响应格式不符合预期：{error}"))
    }
}

impl From<url::ParseError> for AppError {
    fn from(error: url::ParseError) -> Self {
        AppError::Invalid(error.to_string())
    }
}

pub type Result<T, E = AppError> = std::result::Result<T, E>;
