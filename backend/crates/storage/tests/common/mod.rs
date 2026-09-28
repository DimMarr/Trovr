#![allow(dead_code)]

use std::time::Duration;

use testcontainers::core::{ExecCommand, IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

use trovr_storage::{ObjectStorage, PresignedRequest, StorageConfig};

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
const ACCESS_KEY_ID: &str = "GK0123456789abcdef01234567";
const SECRET_ACCESS_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const BUCKET: &str = "trovr-test";

pub struct TestStorage {
    pub storage: ObjectStorage,
    pub config: StorageConfig,
    pub http: reqwest::Client,
    _container: ContainerAsync<GenericImage>,
}

/// Starts a single-node Garage (an S3-compatible backend) with one bucket
/// and one key allowed to use it.
pub async fn start_storage() -> TestStorage {
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
            ACCESS_KEY_ID,
            SECRET_ACCESS_KEY,
        ],
    )
    .await;
    garage(&container, &["bucket", "create", BUCKET]).await;
    garage(
        &container,
        &[
            "bucket",
            "allow",
            "--read",
            "--write",
            "--owner",
            BUCKET,
            "--key",
            ACCESS_KEY_ID,
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
        bucket: BUCKET.to_string(),
        access_key_id: ACCESS_KEY_ID.to_string(),
        secret_access_key: SECRET_ACCESS_KEY.to_string(),
        force_path_style: true,
    };

    TestStorage {
        storage: ObjectStorage::new(&config),
        config,
        http: reqwest::Client::new(),
        _container: container,
    }
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

/// Sends a presigned request the way a browser would.
pub async fn send(
    http: &reqwest::Client,
    request: &PresignedRequest,
    body: Vec<u8>,
) -> reqwest::Response {
    let method = reqwest::Method::from_bytes(request.method.as_bytes()).expect("valid HTTP method");
    let mut builder = http.request(method, &request.url);
    for (name, value) in &request.headers {
        builder = builder.header(name, value);
    }
    builder
        .body(body)
        .send()
        .await
        .expect("request should reach the storage backend")
}

pub async fn upload(test: &TestStorage, key: &str, body: &[u8]) {
    let request = test
        .storage
        .presign_upload(key, "application/octet-stream", Duration::from_secs(60))
        .await
        .expect("upload should presign");
    let response = send(&test.http, &request, body.to_vec()).await;
    assert!(
        response.status().is_success(),
        "upload failed: {}",
        response.status()
    );
}
