//! S3 client wrapper and presigned URL generation.

mod config;
mod disposition;
mod error;
mod storage;

pub use config::StorageConfig;
pub use error::StorageError;
pub use storage::{ObjectInfo, ObjectStorage, PresignedRequest};
