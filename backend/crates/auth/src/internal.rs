use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{AuthError, AuthenticatedUser, TokenValidator};

const INTERNAL_ISSUER: &str = "internal";
const TOKEN_TTL_SECONDS: i64 = 3600;

#[derive(Debug, Serialize, Deserialize)]
struct InternalClaims {
    sub: String,
    iss: String,
    email: String,
    name: String,
    iat: i64,
    exp: i64,
}

pub struct InternalValidator {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl InternalValidator {
    pub fn new(private_key_pem: &str, public_key_pem: &str) -> Result<Self, AuthError> {
        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())?;
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())?;
        Ok(Self {
            encoding_key,
            decoding_key,
        })
    }

    pub async fn create_internal_user(
        &self,
        pool: &PgPool,
        email: &str,
        display_name: &str,
        password: &str,
    ) -> Result<Uuid, AuthError> {
        let email = email.trim().to_lowercase();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::Hash(e.to_string()))?
            .to_string();

        let mut tx = pool.begin().await?;

        let user_id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (issuer, subject, email, display_name) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(INTERNAL_ISSUER)
        .bind(&email)
        .bind(&email)
        .bind(display_name)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO local_credentials (user_id, password_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(user_id)
    }

    pub async fn login(
        &self,
        pool: &PgPool,
        email: &str,
        password: &str,
    ) -> Result<String, AuthError> {
        let email = email.trim().to_lowercase();
        let row: Option<(Uuid, String, String, String)> = sqlx::query_as(
            "SELECT u.id, u.email, u.display_name, c.password_hash \
             FROM users u JOIN local_credentials c ON c.user_id = u.id \
             WHERE u.issuer = $1 AND u.email = $2 AND u.is_active",
        )
        .bind(INTERNAL_ISSUER)
        .bind(&email)
        .fetch_optional(pool)
        .await?;

        let (user_id, email, display_name, password_hash) =
            row.ok_or(AuthError::InvalidCredentials)?;

        let parsed_hash =
            PasswordHash::new(&password_hash).map_err(|e| AuthError::Hash(e.to_string()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AuthError::InvalidCredentials)?;

        self.issue_token(user_id, &email, &display_name)
    }

    pub fn issue_token(
        &self,
        user_id: Uuid,
        email: &str,
        display_name: &str,
    ) -> Result<String, AuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_secs() as i64;

        let claims = InternalClaims {
            sub: user_id.to_string(),
            iss: INTERNAL_ISSUER.to_string(),
            email: email.to_string(),
            name: display_name.to_string(),
            iat: now,
            exp: now + TOKEN_TTL_SECONDS,
        };

        let token = encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)?;
        Ok(token)
    }
}

#[async_trait]
impl TokenValidator for InternalValidator {
    async fn validate(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[INTERNAL_ISSUER]);

        let token_data = decode::<InternalClaims>(token, &self.decoding_key, &validation)?;
        let claims = token_data.claims;

        let user_id =
            Uuid::parse_str(&claims.sub).map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(AuthenticatedUser {
            user_id,
            issuer: claims.iss,
            subject: claims.sub,
            email: claims.email,
            display_name: claims.name,
        })
    }
}
