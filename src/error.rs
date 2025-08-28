use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("API request failed: {0}")]
    ApiError(#[from] reqwest::Error),

    #[error("JSON deserialization failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Data parsing error: {0}")]
    ParseError(String),

    #[error("Missing expected data: {0}")]
    MissingData(String),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    #[error("3D model conversion failed: {0}")]
    ModelConversionError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
