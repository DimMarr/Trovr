use trovr_config::{AppConfig, AuthMode, LogFormat};

const JWT_PRIVATE_KEY_PEM: &str = include_str!("../../api/tests/fixtures/jwt_private.pem");
const JWT_PUBLIC_KEY_PEM: &str = include_str!("../../api/tests/fixtures/jwt_public.pem");

fn config(auth_mode: AuthMode) -> AppConfig {
    AppConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        database_url: "postgres://trovr:trovr@127.0.0.1:1/trovr".to_string(),
        log_format: LogFormat::Pretty,
        auth_mode,
        jwt_private_key_pem: JWT_PRIVATE_KEY_PEM.to_string(),
        jwt_public_key_pem: JWT_PUBLIC_KEY_PEM.to_string(),
        oidc_issuer_url: None,
        oidc_client_id: None,
        s3_endpoint_url: Some("http://127.0.0.1:9".to_string()),
        s3_public_endpoint_url: None,
        s3_region: "us-east-1".to_string(),
        s3_bucket: "trovr".to_string(),
        s3_access_key_id: "access".to_string(),
        s3_secret_access_key: "secret".to_string(),
        s3_force_path_style: true,
        auth_allow_registration: true,
        max_upload_bytes: 1024,
        upload_url_ttl_seconds: 120,
        download_url_ttl_seconds: 30,
    }
}

fn lazy_pool() -> sqlx::PgPool {
    // Never connects: building the state must not need a database.
    sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://trovr:trovr@127.0.0.1:1/trovr")
        .expect("lazy pool")
}

#[tokio::test]
async fn auth_modes_enable_the_matching_validators() {
    let internal = trovr::build_state(&config(AuthMode::Internal), lazy_pool()).unwrap();
    assert!(internal.internal_auth.is_some());
    assert!(internal.oidc_auth.is_none());

    let mut both_config = config(AuthMode::Both);
    both_config.oidc_issuer_url = Some("https://idp.example.com".to_string());
    both_config.oidc_client_id = Some("trovr".to_string());
    let both = trovr::build_state(&both_config, lazy_pool()).unwrap();
    assert!(both.internal_auth.is_some());
    assert!(both.oidc_auth.is_some());

    let mut oidc_config = config(AuthMode::Oidc);
    oidc_config.oidc_issuer_url = Some("https://idp.example.com".to_string());
    oidc_config.oidc_client_id = Some("trovr".to_string());
    let oidc = trovr::build_state(&oidc_config, lazy_pool()).unwrap();
    assert!(oidc.internal_auth.is_none());
    assert!(oidc.oidc_auth.is_some());
}

#[tokio::test]
async fn oidc_modes_require_issuer_and_client_id() {
    let error = trovr::build_state(&config(AuthMode::Oidc), lazy_pool())
        .err()
        .expect("missing OIDC settings must fail");

    assert!(
        error.to_string().contains("APP__OIDC_ISSUER_URL"),
        "{error}"
    );
}

#[tokio::test]
async fn settings_are_mapped_from_the_configuration() {
    let state = trovr::build_state(&config(AuthMode::Internal), lazy_pool()).unwrap();

    assert!(state.settings.allow_registration);
    assert_eq!(state.settings.max_upload_bytes, 1024);
    assert_eq!(state.settings.upload_url_ttl.as_secs(), 120);
    assert_eq!(state.settings.download_url_ttl.as_secs(), 30);
}

#[tokio::test]
async fn unparsable_jwt_keys_are_rejected_at_startup() {
    let mut broken = config(AuthMode::Internal);
    broken.jwt_private_key_pem = "not a pem".to_string();

    assert!(trovr::build_state(&broken, lazy_pool()).is_err());
}
