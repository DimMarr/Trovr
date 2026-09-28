use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("invalid presigned URL lifetime: {0}")]
    InvalidExpiry(String),
    #[error("object storage request failed: {0}")]
    Request(String),
}
