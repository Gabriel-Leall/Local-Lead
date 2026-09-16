use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("invalid api key")]
    InvalidApiKey,
    #[error("rate limited, retry later")]
    RateLimit,
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("missing api key — configure it in Settings")]
    MissingApiKey,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

impl From<AppError> for tauri::ipc::InvokeError {
    fn from(e: AppError) -> Self {
        tauri::ipc::InvokeError::from(e.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            return Self::Network("timeout — check your connection".into());
        }
        Self::Network(e.to_string())
    }
}
