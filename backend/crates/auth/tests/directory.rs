use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use trovr_auth::{find_users_by_email, find_users_by_ids};

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
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should apply cleanly");
    (pool, container)
}

async fn insert_user(pool: &PgPool, issuer: &str, email: &str, active: bool) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (issuer, subject, email, display_name, is_active) \
         VALUES ($1, $2, $2, $2, $3) RETURNING id",
    )
    .bind(issuer)
    .bind(email)
    .bind(active)
    .fetch_one(pool)
    .await
    .expect("user insert should succeed")
}

#[tokio::test]
async fn users_are_found_by_normalized_email() {
    let (pool, _container) = start_migrated_postgres().await;
    let alice = insert_user(&pool, "internal", "alice@example.com", true).await;
    insert_user(&pool, "internal", "gone@example.com", false).await;

    let found = find_users_by_email(&pool, "  Alice@Example.COM ")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, alice);
    assert_eq!(found[0].email, "alice@example.com");

    assert!(
        find_users_by_email(&pool, "gone@example.com")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        find_users_by_email(&pool, "nobody@example.com")
            .await
            .unwrap()
            .is_empty()
    );

    insert_user(&pool, "https://idp.example.com", "alice@example.com", true).await;
    assert_eq!(
        find_users_by_email(&pool, "alice@example.com")
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn profiles_are_loaded_by_id() {
    let (pool, _container) = start_migrated_postgres().await;
    let alice = insert_user(&pool, "internal", "alice@example.com", true).await;
    let bob = insert_user(&pool, "internal", "bob@example.com", true).await;

    let mut profiles = find_users_by_ids(&pool, &[alice, bob, Uuid::nil()])
        .await
        .unwrap();
    profiles.sort_by(|a, b| a.email.cmp(&b.email));

    let emails: Vec<&str> = profiles.iter().map(|p| p.email.as_str()).collect();
    assert_eq!(emails, vec!["alice@example.com", "bob@example.com"]);
    assert_eq!(profiles[1].id, bob);
}
