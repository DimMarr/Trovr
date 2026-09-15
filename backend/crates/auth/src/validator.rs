use async_trait::async_trait;

use crate::{AuthError, AuthenticatedUser};

#[async_trait]
pub trait TokenValidator: Send + Sync {
    async fn validate(&self, token: &str) -> Result<AuthenticatedUser, AuthError>;
}
