use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("token error: {0}")]
    Token(#[from] jsonwebtoken::errors::Error),
    #[error("password hashing error: {0}")]
    Hash(String),
    #[error("oidc discovery error: {0}")]
    Discovery(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_credentials_has_a_stable_message() {
        assert_eq!(
            AuthError::InvalidCredentials.to_string(),
            "invalid credentials"
        );
    }

    #[test]
    fn invalid_token_includes_the_detail() {
        let err = AuthError::InvalidToken("missing kid".to_string());
        assert_eq!(err.to_string(), "invalid token: missing kid");
    }
}
