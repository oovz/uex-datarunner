use foundry_local_sdk::FoundryLocalError;
use serde::Serialize;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Foundry Local error: {0}")]
    Foundry(#[from] FoundryLocalError),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub(crate) type AppResult<T> = Result<T, AppError>;
