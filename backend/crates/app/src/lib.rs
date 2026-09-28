use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use tracing_subscriber::EnvFilter;
use trovr_api::{ApiSettings, AppState};
use trovr_auth::{InternalValidator, OidcValidator};
use trovr_config::{AppConfig, AuthMode, LogFormat};
use trovr_metadata::NodeStore;
use trovr_storage::{ObjectStorage, StorageConfig};

pub async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

pub async fn health_check(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("SELECT 1 AS one").fetch_one(pool).await?;
    Ok(())
}

pub fn init_tracing(format: &LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    match format {
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(filter)
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }
}

/// Builds the API state from the configuration: the validators enabled by
/// `auth_mode`, the node store, and the object storage client.
pub fn build_state(config: &AppConfig, pool: sqlx::PgPool) -> anyhow::Result<AppState> {
    let internal_auth = match config.auth_mode {
        AuthMode::Internal | AuthMode::Both => Some(Arc::new(
            InternalValidator::new(&config.jwt_private_key_pem, &config.jwt_public_key_pem)
                .context("invalid APP__JWT_PRIVATE_KEY_PEM or APP__JWT_PUBLIC_KEY_PEM")?,
        )),
        AuthMode::Oidc => None,
    };

    let oidc_auth =
        match config.auth_mode {
            AuthMode::Oidc | AuthMode::Both => {
                let issuer_url = config.oidc_issuer_url.clone().context(
                    "APP__OIDC_ISSUER_URL is required when APP__AUTH_MODE is oidc or both",
                )?;
                let client_id = config.oidc_client_id.clone().context(
                    "APP__OIDC_CLIENT_ID is required when APP__AUTH_MODE is oidc or both",
                )?;
                Some(Arc::new(OidcValidator::new(
                    pool.clone(),
                    issuer_url,
                    client_id,
                )))
            }
            AuthMode::Internal => None,
        };

    let storage = ObjectStorage::new(&StorageConfig {
        endpoint_url: config.s3_endpoint_url.clone(),
        public_endpoint_url: config.s3_public_endpoint_url.clone(),
        region: config.s3_region.clone(),
        bucket: config.s3_bucket.clone(),
        access_key_id: config.s3_access_key_id.clone(),
        secret_access_key: config.s3_secret_access_key.clone(),
        force_path_style: config.s3_force_path_style,
    });

    Ok(AppState {
        nodes: NodeStore::new(pool.clone()),
        pool,
        storage,
        internal_auth,
        oidc_auth,
        settings: ApiSettings {
            allow_registration: config.auth_allow_registration,
            max_upload_bytes: config.max_upload_bytes,
            upload_url_ttl: Duration::from_secs(config.upload_url_ttl_seconds),
            download_url_ttl: Duration::from_secs(config.download_url_ttl_seconds),
        },
    })
}

/// Resolves on Ctrl-C or SIGTERM (what Kubernetes sends before killing a
/// pod), letting in-flight requests finish.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Ctrl-C handler should install");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler should install")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("shutdown signal received, draining connections");
}
