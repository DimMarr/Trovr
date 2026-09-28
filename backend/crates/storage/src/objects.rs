use aws_sdk_s3::types::{Delete, ObjectIdentifier};

use crate::storage::request_error;
use crate::{ObjectInfo, ObjectStorage, StorageError};

/// S3 rejects `DeleteObjects` requests with more keys than this.
const MAX_KEYS_PER_DELETE: usize = 1000;

impl ObjectStorage {
    /// Reads an object's size and content type, e.g. to confirm an upload.
    ///
    /// Returns `Ok(None)` only when the backend reports the object missing;
    /// every other failure (credentials, network, ...) is an error.
    pub async fn head(&self, key: &str) -> Result<Option<ObjectInfo>, StorageError> {
        let output = match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(output) => output,
            Err(err) if err.as_service_error().is_some_and(|err| err.is_not_found()) => {
                return Ok(None);
            }
            Err(err) => return Err(request_error(err)),
        };

        let size_bytes = output.content_length().ok_or_else(|| {
            StorageError::Request(format!("HEAD {key} returned no Content-Length"))
        })?;

        Ok(Some(ObjectInfo {
            size_bytes,
            content_type: output.content_type().map(str::to_string),
        }))
    }

    /// Deletes objects, e.g. the storage keys returned by
    /// `NodeStore::purge`. Keys that no longer exist are not an error.
    pub async fn delete(&self, keys: &[String]) -> Result<(), StorageError> {
        for batch in keys.chunks(MAX_KEYS_PER_DELETE) {
            let objects = batch
                .iter()
                .map(|key| ObjectIdentifier::builder().key(key).build())
                .collect::<Result<Vec<_>, _>>()
                .map_err(request_error)?;
            let delete = Delete::builder()
                .set_objects(Some(objects))
                .quiet(true)
                .build()
                .map_err(request_error)?;

            let output = self
                .client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(delete)
                .send()
                .await
                .map_err(request_error)?;

            if let Some(failure) = output.errors().first() {
                return Err(StorageError::Request(format!(
                    "failed to delete {}: {}",
                    failure.key().unwrap_or("<unknown key>"),
                    failure.message().unwrap_or("unknown error"),
                )));
            }
        }

        Ok(())
    }
}
