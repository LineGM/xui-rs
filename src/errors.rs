use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("ToStr error: {0}")]
    ToStrError(#[from] reqwest::header::ToStrError),

    #[error("Custom error: {0}")]
    CustomError(String),

    #[error("String error: {0}")]
    StrError(String),

    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
}
