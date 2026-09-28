#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use testcontainers::core::{ExecCommand, IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;
use uuid::Uuid;

use trovr_api::{ApiSettings, AppState, router};
use trovr_auth::{InternalValidator, OidcValidator};
use trovr_metadata::NodeStore;
use trovr_storage::{ObjectStorage, StorageConfig};

pub const JWT_PRIVATE_KEY_PEM: &str = include_str!("../fixtures/jwt_private.pem");
pub const JWT_PUBLIC_KEY_PEM: &str = include_str!("../fixtures/jwt_public.pem");
pub const PASSWORD: &str = "correct horse battery staple";
pub const MAX_UPLOAD_BYTES: i64 = 1024;

pub struct TestOptions {
    pub settings: ApiSettings,
    pub internal_auth: bool,
    pub oidc_issuer: Option<String>,
    pub garage: bool,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self {
            settings: ApiSettings {
                allow_registration: true,
                max_upload_bytes: MAX_UPLOAD_BYTES,
                upload_url_ttl: Duration::from_secs(60),
                download_url_ttl: Duration::from_secs(60),
            },
            internal_auth: true,
            oidc_issuer: None,
            garage: false,
        }
    }
}

pub struct TestUser {
    pub id: Uuid,
    pub token: String,
}

#[derive(Debug)]
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
}

pub struct TestApp {
    pub state: AppState,
    pub http: reqwest::Client,
    router: Router,
    _postgres: ContainerAsync<Postgres>,
    _garage: Option<ContainerAsync<GenericImage>>,
}

impl TestApp {
    pub async fn start() -> Self {
        Self::start_with(TestOptions::default()).await
    }

    pub async fn start_with_storage() -> Self {
        Self::start_with(TestOptions {
            garage: true,
            ..TestOptions::default()
        })
        .await
    }

    pub async fn start_with(options: TestOptions) -> Self {
        let (pool, postgres) = start_postgres().await;
        let (storage_config, garage) = if options.garage {
            let (config, container) = start_garage().await;
            (config, Some(container))
        } else {
            (unreachable_storage(), None)
        };

        let internal_auth = options.internal_auth.then(|| {
            Arc::new(
                InternalValidator::new(JWT_PRIVATE_KEY_PEM, JWT_PUBLIC_KEY_PEM)
                    .expect("test keys should load"),
            )
        });
        let oidc_auth = options.oidc_issuer.map(|issuer| {
            Arc::new(OidcValidator::new(
                pool.clone(),
                issuer,
                "trovr".to_string(),
            ))
        });

        let state = AppState {
            pool: pool.clone(),
            nodes: NodeStore::new(pool),
            storage: ObjectStorage::new(&storage_config),
            internal_auth,
            oidc_auth,
            settings: options.settings,
        };

        Self {
            router: router(state.clone()),
            state,
            http: reqwest::Client::new(),
            _postgres: postgres,
            _garage: garage,
        }
    }

    /// Creates an internal account directly (bypassing `/auth/register`) and
    /// returns a valid access token for it.
    pub async fn create_user(&self, email: &str) -> TestUser {
        let internal = self
            .state
            .internal_auth
            .as_ref()
            .expect("internal auth is enabled");
        let id = internal
            .create_internal_user(&self.state.pool, email, email, PASSWORD)
            .await
            .expect("user should be created");
        let token = internal
            .issue_token(id, email, email)
            .expect("token should be issued");
        TestUser { id, token }
    }

    pub async fn request(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> TestResponse {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        let request = match body {
            Some(body) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => builder.body(Body::empty()),
        }
        .expect("request should build");

        self.send(request).await
    }

    pub async fn get(&self, uri: &str, token: &str) -> TestResponse {
        self.request(Method::GET, uri, Some(token), None).await
    }

    pub async fn post(&self, uri: &str, token: &str, body: Value) -> TestResponse {
        self.request(Method::POST, uri, Some(token), Some(body))
            .await
    }

    pub async fn delete(&self, uri: &str, token: &str) -> TestResponse {
        self.request(Method::DELETE, uri, Some(token), None).await
    }

    pub async fn get_with_authorization(&self, uri: &str, authorization: &str) -> TestResponse {
        let request = Request::builder()
            .uri(uri)
            .header(AUTHORIZATION, authorization)
            .body(Body::empty())
            .expect("request should build");
        self.send(request).await
    }

    async fn send(&self, request: Request<Body>) -> TestResponse {
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("router should answer");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        TestResponse {
            status,
            headers,
            body,
        }
    }

    /// Uploads `content` the way the web client does: ask the API for a
    /// presigned URL, then PUT the bytes straight to storage. Returns the
    /// storage key to confirm.
    pub async fn upload(&self, user: &TestUser, mime_type: &str, content: &[u8]) -> String {
        let response = self
            .post(
                "/api/v1/uploads",
                &user.token,
                json!({ "size_bytes": content.len(), "mime_type": mime_type }),
            )
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{:?}", response.body);
        self.put_presigned(&response.body["upload"], content).await;
        response.body["storage_key"]
            .as_str()
            .expect("storage key")
            .to_string()
    }

    pub async fn put_presigned(&self, upload: &Value, content: &[u8]) {
        let mut request = self.http.put(upload["url"].as_str().expect("upload url"));
        for (name, value) in upload["headers"].as_object().expect("upload headers") {
            request = request.header(name, value.as_str().expect("header value"));
        }
        let response = request
            .body(content.to_vec())
            .send()
            .await
            .expect("storage should answer");
        assert!(
            response.status().is_success(),
            "upload to storage failed: {}",
            response.status()
        );
    }

    /// Uploads and confirms a file; returns the created node as JSON.
    pub async fn upload_file(
        &self,
        user: &TestUser,
        parent_id: Option<&str>,
        name: &str,
        content: &[u8],
    ) -> Value {
        let storage_key = self.upload(user, "text/plain", content).await;
        let response = self
            .post(
                "/api/v1/files",
                &user.token,
                json!({ "storage_key": storage_key, "parent_id": parent_id, "name": name }),
            )
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{:?}", response.body);
        response.body
    }

    /// Follows a presigned request returned by the API, like a browser.
    pub async fn fetch(&self, presigned: &Value) -> reqwest::Response {
        let mut request = self
            .http
            .get(presigned["url"].as_str().expect("presigned url"));
        for (name, value) in presigned["headers"].as_object().expect("presigned headers") {
            request = request.header(name, value.as_str().expect("header value"));
        }
        request.send().await.expect("storage should answer")
    }
}

async fn start_postgres() -> (PgPool, ContainerAsync<Postgres>) {
    let container = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await
        .expect("postgres container should start");
    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port should be published");
    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("should connect to postgres");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should apply cleanly");

    (pool, container)
}

/// Storage settings pointing at a closed port: presigning works offline, and
/// tests that never start Garage never reach the backend.
fn unreachable_storage() -> StorageConfig {
    StorageConfig {
        endpoint_url: Some("http://127.0.0.1:9".to_string()),
        public_endpoint_url: None,
        region: "us-east-1".to_string(),
        bucket: "trovr".to_string(),
        access_key_id: "test".to_string(),
        secret_access_key: "test".to_string(),
        force_path_style: true,
    }
}

const GARAGE_CONFIG: &str = r#"
metadata_dir = "/var/lib/garage/meta"
data_dir = "/var/lib/garage/data"
db_engine = "sqlite"
replication_factor = 1
rpc_bind_addr = "[::]:3901"
rpc_public_addr = "127.0.0.1:3901"
rpc_secret = "0000000000000000000000000000000000000000000000000000000000000000"

[s3_api]
s3_region = "garage"
api_bind_addr = "[::]:3900"
"#;

// Garage only imports keys in its own format: "GK" + 24 hex chars, and a
// 64 hex chars secret.
const GARAGE_ACCESS_KEY_ID: &str = "GK0123456789abcdef01234567";
const GARAGE_SECRET_ACCESS_KEY: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const GARAGE_BUCKET: &str = "trovr-test";

/// Starts a single-node Garage with one bucket and one key allowed to use it.
async fn start_garage() -> (StorageConfig, ContainerAsync<GenericImage>) {
    let container = GenericImage::new("dxflrs/garage", "v2.4.1")
        .with_exposed_port(3900.tcp())
        .with_wait_for(WaitFor::message_on_stderr("S3 API server listening on"))
        .with_wait_for(WaitFor::message_on_stderr("Listening on [::]:3901"))
        .with_copy_to("/etc/garage.toml", GARAGE_CONFIG.as_bytes().to_vec())
        .start()
        .await
        .expect("garage container should start");

    let node_id = garage(&container, &["node", "id", "-q"]).await;
    let node_id = node_id.trim().split('@').next().expect("node id output");
    garage(
        &container,
        &["layout", "assign", "-z", "dc1", "-c", "1G", node_id],
    )
    .await;
    garage(&container, &["layout", "apply", "--version", "1"]).await;
    garage(
        &container,
        &[
            "key",
            "import",
            "--yes",
            "-n",
            "trovr-test",
            GARAGE_ACCESS_KEY_ID,
            GARAGE_SECRET_ACCESS_KEY,
        ],
    )
    .await;
    garage(&container, &["bucket", "create", GARAGE_BUCKET]).await;
    garage(
        &container,
        &[
            "bucket",
            "allow",
            "--read",
            "--write",
            "--owner",
            GARAGE_BUCKET,
            "--key",
            GARAGE_ACCESS_KEY_ID,
        ],
    )
    .await;

    let host_port = container
        .get_host_port_ipv4(3900)
        .await
        .expect("garage S3 port should be published");

    let config = StorageConfig {
        endpoint_url: Some(format!("http://127.0.0.1:{host_port}")),
        public_endpoint_url: None,
        region: "garage".to_string(),
        bucket: GARAGE_BUCKET.to_string(),
        access_key_id: GARAGE_ACCESS_KEY_ID.to_string(),
        secret_access_key: GARAGE_SECRET_ACCESS_KEY.to_string(),
        force_path_style: true,
    };

    (config, container)
}

/// Runs the `garage` admin CLI inside the container and returns its stdout.
async fn garage(container: &ContainerAsync<GenericImage>, args: &[&str]) -> String {
    let command = std::iter::once("/garage").chain(args.iter().copied());
    let mut result = container
        .exec(ExecCommand::new(command))
        .await
        .expect("garage command should start");
    let stdout = String::from_utf8(result.stdout_to_vec().await.expect("garage stdout"))
        .expect("garage stdout is UTF-8");
    let exit_code = result.exit_code().await.expect("garage exit code");
    assert_eq!(exit_code, Some(0), "garage {args:?} failed: {stdout}");
    stdout
}
