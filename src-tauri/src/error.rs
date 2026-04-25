use std::fmt;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Ai(String),
    Audio(String),
    Http(String),
    Io(String),
    Json(String),
    Secret(String),
    Settings(String),
}

impl AppError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Ai(message)
            | Self::Audio(message)
            | Self::Http(message)
            | Self::Io(message)
            | Self::Json(message)
            | Self::Secret(message)
            | Self::Settings(message) => message.clone(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}

impl std::error::Error for AppError {}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.user_message()
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error.to_string())
    }
}
