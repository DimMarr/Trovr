use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use trovr_auth::{InternalValidator, OidcValidator};
use trovr_metadata::NodeStore;
use trovr_storage::ObjectStorage;

#[derive(Debug, Clone)]
pub struct ApiSettings {
    /// Whether `POST /auth/register` may create internal accounts.
    pub allow_registration: bool,
    pub max_upload_bytes: i64,
    pub upload_url_ttl: Duration,
    pub download_url_ttl: Duration,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub nodes: NodeStore,
    pub storage: ObjectStorage,
    /// `None` when internal accounts are disabled (`auth_mode = oidc`).
    pub internal_auth: Option<Arc<InternalValidator>>,
    /// `None` when external identity providers are disabled
    /// (`auth_mode = internal`).
    pub oidc_auth: Option<Arc<OidcValidator>>,
    pub settings: ApiSettings,
}
