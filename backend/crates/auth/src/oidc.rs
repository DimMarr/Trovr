use async_trait::async_trait;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AuthError, AuthenticatedUser, TokenValidator};

#[derive(Debug, Deserialize)]
struct DiscoveryDocument {
    jwks_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OidcClaims {
    sub: String,
    iss: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
    exp: i64,
    iat: i64,
}

pub struct OidcValidator {
    pool: PgPool,
    issuer_url: String,
    client_id: String,
    http_client: reqwest::Client,
}

impl OidcValidator {
    pub fn new(pool: PgPool, issuer_url: String, client_id: String) -> Self {
        Self {
            pool,
            issuer_url: issuer_url.trim_end_matches('/').to_string(),
            client_id,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("reqwest client should build with a valid default config"),
        }
    }

    /// The issuer, without a trailing slash.
    pub fn issuer_url(&self) -> &str {
        &self.issuer_url
    }

    /// The client id tokens must be issued for (their `aud`).
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    async fn fetch_jwks(&self) -> Result<JwkSet, AuthError> {
        let discovery_url = format!("{}/.well-known/openid-configuration", self.issuer_url);

        let discovery: DiscoveryDocument = self
            .http_client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| AuthError::Discovery(e.to_string()))?
            .json()
            .await
            .map_err(|e| AuthError::Discovery(e.to_string()))?;

        let jwks: JwkSet = self
            .http_client
            .get(&discovery.jwks_uri)
            .send()
            .await
            .map_err(|e| AuthError::Discovery(e.to_string()))?
            .json()
            .await
            .map_err(|e| AuthError::Discovery(e.to_string()))?;

        Ok(jwks)
    }
}

#[async_trait]
impl TokenValidator for OidcValidator {
    async fn validate(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        let header = decode_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| AuthError::InvalidToken("token header is missing a kid".to_string()))?;

        let jwks = self.fetch_jwks().await?;
        let jwk = jwks
            .find(&kid)
            .ok_or_else(|| AuthError::InvalidToken(format!("no matching jwk for kid {kid}")))?;
        let decoding_key = DecodingKey::from_jwk(jwk)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer_url]);
        validation.set_audience(&[&self.client_id]);

        let token_data = decode::<OidcClaims>(token, &decoding_key, &validation)?;
        let claims = token_data.claims;

        let email = claims.email.clone().unwrap_or_default();
        let display_name = claims.name.clone().unwrap_or_else(|| claims.sub.clone());

        let (user_id, is_active): (Uuid, bool) = sqlx::query_as(
            "INSERT INTO users (issuer, subject, email, display_name) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (issuer, subject) DO UPDATE \
             SET email = EXCLUDED.email, display_name = EXCLUDED.display_name, updated_at = now() \
             RETURNING id, is_active",
        )
        .bind(&claims.iss)
        .bind(&claims.sub)
        .bind(&email)
        .bind(&display_name)
        .fetch_one(&self.pool)
        .await?;

        if !is_active {
            return Err(AuthError::InvalidCredentials);
        }

        Ok(AuthenticatedUser {
            user_id,
            issuer: claims.iss,
            subject: claims.sub,
            email,
            display_name,
        })
    }
}
