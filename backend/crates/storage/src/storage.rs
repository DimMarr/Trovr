use std::time::Duration;

use aws_sdk_s3::Client;
use aws_sdk_s3::config::{
    BehaviorVersion, Credentials, Region, RequestChecksumCalculation, ResponseChecksumValidation,
};
use aws_sdk_s3::error::DisplayErrorContext;
use aws_sdk_s3::presigning::{PresignedRequest as SdkPresignedRequest, PresigningConfig};
use uuid::Uuid;

use crate::disposition::attachment_disposition;
use crate::{StorageConfig, StorageError};

/// A presigned HTTP request for the browser to send as-is: every header is
/// part of the signature and must be sent unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresignedRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
}

/// What the backend reports about a stored object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInfo {
    pub size_bytes: i64,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ObjectStorage {
    /// Talks to the backend from the server (`HEAD`, `DeleteObjects`).
    pub(crate) client: Client,
    /// Only signs URLs, for the endpoint browsers use.
    presign_client: Client,
    pub(crate) bucket: String,
}

impl ObjectStorage {
    pub fn new(config: &StorageConfig) -> Self {
        let endpoint_url = non_empty(config.endpoint_url.as_deref());
        let public_endpoint_url = non_empty(config.public_endpoint_url.as_deref()).or(endpoint_url);

        Self {
            client: build_client(config, endpoint_url),
            presign_client: build_client(config, public_endpoint_url),
            bucket: config.bucket.clone(),
        }
    }

    /// Returns a fresh key for an object uploaded by `owner_id`; never
    /// derived from a file name. Scoping keys to their uploader lets the API
    /// check, statelessly, that a client confirms only its own uploads.
    pub fn new_object_key(owner_id: Uuid) -> String {
        format!("users/{owner_id}/{}", Uuid::new_v4())
    }

    /// Whether `key` is exactly a key `new_object_key(owner_id)` could return.
    pub fn key_belongs_to(key: &str, owner_id: Uuid) -> bool {
        key.strip_prefix(&format!("users/{owner_id}/"))
            .and_then(|rest| Uuid::parse_str(rest).ok().map(|id| id.to_string() == rest))
            .unwrap_or(false)
    }

    /// Presigns a PUT the browser uses to upload `key` directly to storage.
    /// The content type is signed: the upload must send exactly this value.
    pub async fn presign_upload(
        &self,
        key: &str,
        content_type: &str,
        expires_in: Duration,
    ) -> Result<PresignedRequest, StorageError> {
        let request = self
            .presign_client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config(expires_in)?)
            .await
            .map_err(request_error)?;

        Ok(into_presigned_request(request))
    }

    /// Presigns a GET that downloads `key` as an attachment named `file_name`.
    pub async fn presign_download(
        &self,
        key: &str,
        file_name: &str,
        expires_in: Duration,
    ) -> Result<PresignedRequest, StorageError> {
        let request = self
            .presign_client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_disposition(attachment_disposition(file_name))
            .presigned(presigning_config(expires_in)?)
            .await
            .map_err(request_error)?;

        Ok(into_presigned_request(request))
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn build_client(config: &StorageConfig, endpoint_url: Option<&str>) -> Client {
    let credentials = Credentials::new(
        &config.access_key_id,
        &config.secret_access_key,
        None,
        None,
        "trovr-config",
    );

    let mut builder = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(config.region.clone()))
        .credentials_provider(credentials)
        .force_path_style(config.force_path_style)
        // The newer "when supported" defaults add CRC32 checksums that several
        // S3-compatible backends reject and that would be baked into presigned
        // PUT URLs a browser cannot satisfy.
        .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
        .response_checksum_validation(ResponseChecksumValidation::WhenRequired);

    if let Some(endpoint_url) = endpoint_url {
        builder = builder.endpoint_url(endpoint_url);
    }

    Client::from_conf(builder.build())
}

fn presigning_config(expires_in: Duration) -> Result<PresigningConfig, StorageError> {
    PresigningConfig::expires_in(expires_in)
        .map_err(|err| StorageError::InvalidExpiry(err.to_string()))
}

fn into_presigned_request(request: SdkPresignedRequest) -> PresignedRequest {
    PresignedRequest {
        method: request.method().to_string(),
        url: request.uri().to_string(),
        headers: request
            .headers()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect(),
    }
}

pub(crate) fn request_error(err: impl std::error::Error) -> StorageError {
    StorageError::Request(DisplayErrorContext(&err).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(endpoint_url: Option<&str>, public_endpoint_url: Option<&str>) -> StorageConfig {
        StorageConfig {
            endpoint_url: endpoint_url.map(str::to_string),
            public_endpoint_url: public_endpoint_url.map(str::to_string),
            region: "us-east-1".to_string(),
            bucket: "trovr".to_string(),
            access_key_id: "access".to_string(),
            secret_access_key: "secret".to_string(),
            force_path_style: true,
        }
    }

    fn header<'a>(request: &'a PresignedRequest, name: &str) -> Option<&'a str> {
        request
            .headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn object_keys_are_unique_and_scoped_to_their_owner() {
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();

        let first = ObjectStorage::new_object_key(owner);
        let second = ObjectStorage::new_object_key(owner);

        assert_ne!(first, second);
        assert!(first.starts_with(&format!("users/{owner}/")));
        assert!(ObjectStorage::key_belongs_to(&first, owner));
        assert!(!ObjectStorage::key_belongs_to(&first, other));
    }

    #[test]
    fn only_well_formed_keys_belong_to_anyone() {
        let owner = Uuid::new_v4();
        let id = Uuid::new_v4();

        for key in [
            format!("users/{owner}/"),
            format!("users/{owner}/not-a-uuid"),
            format!("users/{owner}/{id}/extra"),
            format!("users/{owner}/../{id}"),
            format!("users/{owner}/{}", id.simple()),
            format!("/users/{owner}/{id}"),
            format!("objects/{id}"),
        ] {
            assert!(!ObjectStorage::key_belongs_to(&key, owner), "{key}");
        }
    }

    #[tokio::test]
    async fn presigned_urls_use_the_public_endpoint() {
        let storage = ObjectStorage::new(&config(
            Some("http://minio:9000"),
            Some("https://files.example.com"),
        ));

        let upload = storage
            .presign_upload("objects/abc", "image/png", Duration::from_secs(300))
            .await
            .unwrap();

        assert_eq!(upload.method, "PUT");
        assert!(
            upload
                .url
                .starts_with("https://files.example.com/trovr/objects/abc?"),
            "unexpected url {}",
            upload.url
        );
        assert!(upload.url.contains("X-Amz-Expires=300"));
        assert_eq!(header(&upload, "content-type"), Some("image/png"));
        assert!(
            !upload.url.to_ascii_lowercase().contains("checksum"),
            "a checksum baked into the URL cannot be satisfied by a browser: {}",
            upload.url
        );
    }

    #[tokio::test]
    async fn empty_public_endpoint_falls_back_to_the_endpoint() {
        let storage = ObjectStorage::new(&config(Some("http://minio:9000"), Some("")));

        let download = storage
            .presign_download("objects/abc", "report.pdf", Duration::from_secs(60))
            .await
            .unwrap();

        assert_eq!(download.method, "GET");
        assert!(
            download
                .url
                .starts_with("http://minio:9000/trovr/objects/abc?"),
            "unexpected url {}",
            download.url
        );
        assert!(
            download
                .url
                .contains("response-content-disposition=attachment")
        );
    }

    #[tokio::test]
    async fn lifetimes_over_seven_days_are_rejected() {
        let storage = ObjectStorage::new(&config(Some("http://minio:9000"), None));

        let result = storage
            .presign_upload(
                "objects/abc",
                "text/plain",
                Duration::from_secs(8 * 24 * 3600),
            )
            .await;

        assert!(matches!(result, Err(StorageError::InvalidExpiry(_))));
    }
}
