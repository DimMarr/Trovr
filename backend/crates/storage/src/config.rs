/// Connection settings for an S3-compatible backend.
#[derive(Clone)]
pub struct StorageConfig {
    /// How the server reaches the backend; `None` means AWS S3.
    pub endpoint_url: Option<String>,
    /// How browsers reach the backend, when that differs from `endpoint_url`.
    pub public_endpoint_url: Option<String>,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub force_path_style: bool,
}

impl std::fmt::Debug for StorageConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageConfig")
            .field("endpoint_url", &self.endpoint_url)
            .field("public_endpoint_url", &self.public_endpoint_url)
            .field("region", &self.region)
            .field("bucket", &self.bucket)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("force_path_style", &self.force_path_style)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_redacts_the_secret_key() {
        let config = StorageConfig {
            endpoint_url: None,
            public_endpoint_url: None,
            region: "us-east-1".to_string(),
            bucket: "trovr".to_string(),
            access_key_id: "visible-key-id".to_string(),
            secret_access_key: "super-secret".to_string(),
            force_path_style: true,
        };

        let debug_output = format!("{config:?}");

        assert!(!debug_output.contains("super-secret"));
        assert!(debug_output.contains("visible-key-id"));
    }
}
