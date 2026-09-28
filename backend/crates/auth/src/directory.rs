//! Read-only lookups over `users` for other crates (e.g. to share a node by
//! email), since `trovr-auth` owns that table.

use sqlx::PgPool;
use uuid::Uuid;

use crate::AuthError;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

/// Active users with this email, from any issuer (the same address can exist
/// once per identity provider).
pub async fn find_users_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Vec<UserProfile>, AuthError> {
    let email = email.trim().to_lowercase();
    let users = sqlx::query_as::<_, UserProfile>(
        "SELECT id, email, display_name FROM users \
         WHERE lower(email) = $1 AND is_active ORDER BY created_at, id",
    )
    .bind(email)
    .fetch_all(pool)
    .await?;

    Ok(users)
}

/// Profiles for the given ids; unknown ids are skipped.
pub async fn find_users_by_ids(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<UserProfile>, AuthError> {
    let users = sqlx::query_as::<_, UserProfile>(
        "SELECT id, email, display_name FROM users WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    Ok(users)
}
