use sqlx::PgPool;
use testcontainers::ContainerAsync;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

async fn start_migrated_postgres() -> (PgPool, ContainerAsync<Postgres>) {
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

    trovr::run_migrations(&pool)
        .await
        .expect("migrations should apply cleanly");

    (pool, container)
}

async fn insert_user(pool: &PgPool, subject: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (issuer, subject, email, display_name) VALUES ('internal', $1, $2, $2) RETURNING id",
    )
    .bind(subject)
    .bind(format!("{subject}@example.com"))
    .fetch_one(pool)
    .await
    .expect("user insert should succeed")
}

#[tokio::test]
async fn migrations_create_expected_schema() {
    let (pool, _container) = start_migrated_postgres().await;

    trovr::health_check(&pool)
        .await
        .expect("health check query should succeed");

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name",
    )
    .fetch_all(&pool)
    .await
    .expect("should list tables");

    assert_eq!(
        tables,
        vec![
            "_sqlx_migrations".to_string(),
            "file_versions".to_string(),
            "local_credentials".to_string(),
            "node_permissions".to_string(),
            "nodes".to_string(),
            "users".to_string(),
        ]
    );
}

#[tokio::test]
async fn nodes_enforce_unique_name_per_parent() {
    let (pool, _container) = start_migrated_postgres().await;
    let user_id = insert_user(&pool, "alice").await;

    // A NULL parent_id does not collide in Postgres's unique-constraint
    // semantics (see spec caveat #1), so this test uses a real, non-null
    // parent to actually exercise the constraint.
    let root_id: Uuid = sqlx::query_scalar(
        "INSERT INTO nodes (parent_id, type, name, owner_id) VALUES (NULL, 'folder', 'root', $1) RETURNING id",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("root folder insert should succeed");

    sqlx::query(
        "INSERT INTO nodes (parent_id, type, name, owner_id) VALUES ($1, 'folder', 'Documents', $2)",
    )
    .bind(root_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("first child insert should succeed");

    let duplicate = sqlx::query(
        "INSERT INTO nodes (parent_id, type, name, owner_id) VALUES ($1, 'folder', 'Documents', $2)",
    )
    .bind(root_id)
    .bind(user_id)
    .execute(&pool)
    .await;

    assert!(duplicate.is_err());
}

#[tokio::test]
async fn files_require_a_mime_type() {
    let (pool, _container) = start_migrated_postgres().await;
    let user_id = insert_user(&pool, "bob").await;

    let result = sqlx::query(
        "INSERT INTO nodes (parent_id, type, name, owner_id, mime_type) VALUES (NULL, 'file', 'photo.png', $1, NULL)",
    )
    .bind(user_id)
    .execute(&pool)
    .await;

    assert!(result.is_err());
}
