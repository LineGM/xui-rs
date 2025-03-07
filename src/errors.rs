use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("ToStr error: {0}")]
    ToStrError(#[from] reqwest::header::ToStrError),

    #[error("Custom error: {0}")]
    Custom(String),

    #[error("String error: {0}")]
    Str(String),

    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}
