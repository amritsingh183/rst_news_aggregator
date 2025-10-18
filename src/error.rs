use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AppError {
    #[error("HTTP request failed for {url}: {message}")]
    HttpError { url: String, message: String },

    #[error("Failed to parse response from {origin}: {message}")]
    ParseError { origin: String, message: String },

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Shutdown requested")]
    ShutdownError,

    #[error("No articles found from source: {0}")]
    NoArticlesError(String),

    #[error("Analyzer error: {0}")]
    AnalyzerError(String),
}

impl AppError {
    pub fn http_error(url: impl Into<String>, err: impl std::fmt::Display) -> Self {
        Self::HttpError {
            url: url.into(),
            message: err.to_string(),
        }
    }

    pub fn parse_error(origin: impl Into<String>, err: impl std::fmt::Display) -> Self {
        Self::ParseError {
            origin: origin.into(),
            message: err.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
