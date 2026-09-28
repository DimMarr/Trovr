#![allow(dead_code)]

use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use trovr_metadata::{NewContent, NewFile, NodeStore};

pub struct TestStore {
    pub store: NodeStore,
    pub pool: PgPool,
    _container: ContainerAsync<Postgres>,
}

pub async fn start_store() -> TestStore {
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

    TestStore {
        store: NodeStore::new(pool.clone()),
        pool,
        _container: container,
    }
}

pub async fn insert_user(pool: &PgPool, subject: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (issuer, subject, email, display_name) VALUES ('internal', $1, $2, $1) RETURNING id",
    )
    .bind(subject)
    .bind(format!("{subject}@example.com"))
    .fetch_one(pool)
    .await
    .expect("user insert should succeed")
}

pub fn content(storage_key: &str, size_bytes: i64) -> NewContent {
    NewContent {
        storage_key: storage_key.to_string(),
        size_bytes,
        checksum_sha256: None,
    }
}

pub fn new_file(owner_id: Uuid, parent_id: Option<Uuid>, name: &str, storage_key: &str) -> NewFile {
    NewFile {
        owner_id,
        parent_id,
        name: name.to_string(),
        mime_type: "text/plain".to_string(),
        content: content(storage_key, 42),
    }
}
