use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    Internal,
    Oidc,
    Both,
}

fn default_log_format() -> LogFormat {
    LogFormat::Pretty
}

fn default_auth_mode() -> AuthMode {
    AuthMode::Internal
}

fn default_s3_region() -> String {
    "us-east-1".to_string()
}

fn default_s3_force_path_style() -> bool {
    true
}

fn default_max_upload_bytes() -> i64 {
    5_368_709_120
}

fn default_upload_url_ttl_seconds() -> u64 {
    900
}

fn default_download_url_ttl_seconds() -> u64 {
    300
}

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub bind_addr: String,
    pub database_url: String,
    #[serde(default = "default_log_format")]
    pub log_format: LogFormat,
    #[serde(default = "default_auth_mode")]
    pub auth_mode: AuthMode,
    pub jwt_private_key_pem: String,
    pub jwt_public_key_pem: String,
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    /// How the server reaches the S3-compatible backend; `None` means AWS S3.
    pub s3_endpoint_url: Option<String>,
    /// How browsers reach the backend, when that differs from
    /// `s3_endpoint_url` (presigned URLs embed the host they were signed for).
    pub s3_public_endpoint_url: Option<String>,
    #[serde(default = "default_s3_region")]
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key_id: String,
    pub s3_secret_access_key: String,
    #[serde(default = "default_s3_force_path_style")]
    pub s3_force_path_style: bool,
    /// Whether anyone may create an internal account via the API.
    #[serde(default)]
    pub auth_allow_registration: bool,
    /// Largest accepted upload; a single presigned PUT is capped at 5 GiB.
    #[serde(default = "default_max_upload_bytes")]
    pub max_upload_bytes: i64,
    #[serde(default = "default_upload_url_ttl_seconds")]
    pub upload_url_ttl_seconds: u64,
    #[serde(default = "default_download_url_ttl_seconds")]
    pub download_url_ttl_seconds: u64,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("bind_addr", &self.bind_addr)
            .field("database_url", &"<redacted>")
            .field("log_format", &self.log_format)
            .field("auth_mode", &self.auth_mode)
            .field("jwt_private_key_pem", &"<redacted>")
            .field("jwt_public_key_pem", &"<redacted>")
            .field("oidc_issuer_url", &self.oidc_issuer_url)
            .field("oidc_client_id", &self.oidc_client_id)
            .field("s3_endpoint_url", &self.s3_endpoint_url)
            .field("s3_public_endpoint_url", &self.s3_public_endpoint_url)
            .field("s3_region", &self.s3_region)
            .field("s3_bucket", &self.s3_bucket)
            .field("s3_access_key_id", &self.s3_access_key_id)
            .field("s3_secret_access_key", &"<redacted>")
            .field("s3_force_path_style", &self.s3_force_path_style)
            .field("auth_allow_registration", &self.auth_allow_registration)
            .field("max_upload_bytes", &self.max_upload_bytes)
            .field("upload_url_ttl_seconds", &self.upload_url_ttl_seconds)
            .field("download_url_ttl_seconds", &self.download_url_ttl_seconds)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to load configuration: {0}")]
    Load(#[from] config::ConfigError),
}

impl AppConfig {
    /// Loads configuration from environment variables prefixed with `APP__`,
    /// e.g. `APP__BIND_ADDR`, `APP__DATABASE_URL`, `APP__LOG_FORMAT`,
    /// `APP__AUTH_MODE`, `APP__JWT_PRIVATE_KEY_PEM`, `APP__JWT_PUBLIC_KEY_PEM`,
    /// `APP__OIDC_ISSUER_URL`, `APP__OIDC_CLIENT_ID`, `APP__S3_ENDPOINT_URL`,
    /// `APP__S3_PUBLIC_ENDPOINT_URL`, `APP__S3_REGION`, `APP__S3_BUCKET`,
    /// `APP__S3_ACCESS_KEY_ID`, `APP__S3_SECRET_ACCESS_KEY`,
    /// `APP__S3_FORCE_PATH_STYLE`, `APP__AUTH_ALLOW_REGISTRATION`,
    /// `APP__MAX_UPLOAD_BYTES`, `APP__UPLOAD_URL_TTL_SECONDS`,
    /// `APP__DOWNLOAD_URL_TTL_SECONDS`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let settings = config::Config::builder()
            .set_default("log_format", "pretty")?
            .set_default("auth_mode", "internal")?
            .set_default("s3_region", "us-east-1")?
            .set_default("s3_force_path_style", true)?
            .set_default("auth_allow_registration", false)?
            .set_default("max_upload_bytes", default_max_upload_bytes())?
            .set_default("upload_url_ttl_seconds", default_upload_url_ttl_seconds())?
            .set_default(
                "download_url_ttl_seconds",
                default_download_url_ttl_seconds(),
            )?
            .add_source(config::Environment::default().prefix("APP").separator("__"))
            .build()?;

        Ok(settings.try_deserialize()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_env() {
        unsafe {
            std::env::remove_var("APP__BIND_ADDR");
            std::env::remove_var("APP__DATABASE_URL");
            std::env::remove_var("APP__LOG_FORMAT");
            std::env::remove_var("APP__AUTH_MODE");
            std::env::remove_var("APP__JWT_PRIVATE_KEY_PEM");
            std::env::remove_var("APP__JWT_PUBLIC_KEY_PEM");
            std::env::remove_var("APP__OIDC_ISSUER_URL");
            std::env::remove_var("APP__OIDC_CLIENT_ID");
            std::env::remove_var("APP__S3_ENDPOINT_URL");
            std::env::remove_var("APP__S3_PUBLIC_ENDPOINT_URL");
            std::env::remove_var("APP__S3_REGION");
            std::env::remove_var("APP__S3_BUCKET");
            std::env::remove_var("APP__S3_ACCESS_KEY_ID");
            std::env::remove_var("APP__S3_SECRET_ACCESS_KEY");
            std::env::remove_var("APP__S3_FORCE_PATH_STYLE");
            std::env::remove_var("APP__AUTH_ALLOW_REGISTRATION");
            std::env::remove_var("APP__MAX_UPLOAD_BYTES");
            std::env::remove_var("APP__UPLOAD_URL_TTL_SECONDS");
            std::env::remove_var("APP__DOWNLOAD_URL_TTL_SECONDS");
        }
    }

    fn set_required_env() {
        unsafe {
            std::env::set_var("APP__BIND_ADDR", "0.0.0.0:8080");
            std::env::set_var("APP__DATABASE_URL", "postgres://user:pass@localhost/trovr");
            std::env::set_var("APP__JWT_PRIVATE_KEY_PEM", "fake-private-key-pem");
            std::env::set_var("APP__JWT_PUBLIC_KEY_PEM", "fake-public-key-pem");
            std::env::set_var("APP__S3_BUCKET", "trovr");
            std::env::set_var("APP__S3_ACCESS_KEY_ID", "fake-access-key");
            std::env::set_var("APP__S3_SECRET_ACCESS_KEY", "fake-secret-key");
        }
    }

    #[test]
    #[serial]
    fn from_env_reads_all_fields_from_environment() {
        clear_env();
        set_required_env();
        unsafe {
            std::env::set_var("APP__LOG_FORMAT", "json");
            std::env::set_var("APP__AUTH_MODE", "both");
            std::env::set_var("APP__OIDC_ISSUER_URL", "https://idp.example.com");
            std::env::set_var("APP__OIDC_CLIENT_ID", "trovr-client");
            std::env::set_var("APP__S3_ENDPOINT_URL", "http://minio:9000");
            std::env::set_var("APP__S3_PUBLIC_ENDPOINT_URL", "https://files.example.com");
            std::env::set_var("APP__S3_REGION", "garage");
            std::env::set_var("APP__S3_FORCE_PATH_STYLE", "false");
            std::env::set_var("APP__AUTH_ALLOW_REGISTRATION", "true");
            std::env::set_var("APP__MAX_UPLOAD_BYTES", "1048576");
            std::env::set_var("APP__UPLOAD_URL_TTL_SECONDS", "120");
            std::env::set_var("APP__DOWNLOAD_URL_TTL_SECONDS", "30");
        }

        let config = AppConfig::from_env().expect("config should load");

        assert_eq!(config.bind_addr, "0.0.0.0:8080");
        assert_eq!(config.database_url, "postgres://user:pass@localhost/trovr");
        assert_eq!(config.log_format, LogFormat::Json);
        assert_eq!(config.auth_mode, AuthMode::Both);
        assert_eq!(config.jwt_private_key_pem, "fake-private-key-pem");
        assert_eq!(config.jwt_public_key_pem, "fake-public-key-pem");
        assert_eq!(
            config.oidc_issuer_url,
            Some("https://idp.example.com".to_string())
        );
        assert_eq!(config.oidc_client_id, Some("trovr-client".to_string()));
        assert_eq!(
            config.s3_endpoint_url,
            Some("http://minio:9000".to_string())
        );
        assert_eq!(
            config.s3_public_endpoint_url,
            Some("https://files.example.com".to_string())
        );
        assert_eq!(config.s3_region, "garage");
        assert_eq!(config.s3_bucket, "trovr");
        assert_eq!(config.s3_access_key_id, "fake-access-key");
        assert_eq!(config.s3_secret_access_key, "fake-secret-key");
        assert!(!config.s3_force_path_style);
        assert!(config.auth_allow_registration);
        assert_eq!(config.max_upload_bytes, 1_048_576);
        assert_eq!(config.upload_url_ttl_seconds, 120);
        assert_eq!(config.download_url_ttl_seconds, 30);

        clear_env();
    }

    #[test]
    #[serial]
    fn from_env_defaults_log_format_and_auth_mode_when_unset() {
        clear_env();
        set_required_env();

        let config = AppConfig::from_env().expect("config should load");

        assert_eq!(config.log_format, LogFormat::Pretty);
        assert_eq!(config.auth_mode, AuthMode::Internal);
        assert_eq!(config.oidc_issuer_url, None);
        assert_eq!(config.oidc_client_id, None);
        assert_eq!(config.s3_endpoint_url, None);
        assert_eq!(config.s3_public_endpoint_url, None);
        assert_eq!(config.s3_region, "us-east-1");
        assert!(config.s3_force_path_style);
        assert!(!config.auth_allow_registration);
        assert_eq!(config.max_upload_bytes, 5_368_709_120);
        assert_eq!(config.upload_url_ttl_seconds, 900);
        assert_eq!(config.download_url_ttl_seconds, 300);

        clear_env();
    }

    #[test]
    #[serial]
    fn from_env_errors_when_required_fields_are_missing() {
        clear_env();

        let result = AppConfig::from_env();

        assert!(result.is_err());

        clear_env();
    }

    #[test]
    #[serial]
    fn debug_output_redacts_secrets() {
        clear_env();
        set_required_env();

        let config = AppConfig::from_env().expect("config should load");
        let debug_output = format!("{config:?}");

        assert!(!debug_output.contains("fake-private-key-pem"));
        assert!(!debug_output.contains("fake-public-key-pem"));
        assert!(!debug_output.contains("user:pass"));
        assert!(!debug_output.contains("fake-secret-key"));
        assert!(debug_output.contains("fake-access-key"));
        assert!(debug_output.contains("<redacted>"));

        clear_env();
    }
}
