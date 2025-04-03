use reqwest::Error as ReqwestError;
use reqwest_eventsource::Error as EventSourceError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Network request failed: {0}")]
    RequestError(#[from] ReqwestError),

    #[error("JSON serialization/deserialization error: {0}")]
    SerdeError(#[from] SerdeJsonError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Operation timed out")]
    Timeout(#[from] Elapsed),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Content not found in response: Expected at {0}")]
    ContentNotFound(String),

    #[error("EventSourceError: {0}")]
    EventSourceError(#[from] EventSourceError),

    #[error("Error: {0}")]
    OtherError(String),

    #[error("Any error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}
