use async_trait::async_trait;

use crate::{AuthError, AuthenticatedUser};

#[async_trait]
pub trait TokenValidator: Send + Sync {
    async fn validate(&self, token: &str) -> Result<AuthenticatedUser, AuthError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InternalValidator, OidcValidator};

    fn _assert_both_validators_are_object_safe(
        internal: InternalValidator,
        oidc: OidcValidator,
    ) -> Vec<Box<dyn TokenValidator>> {
        vec![Box::new(internal), Box::new(oidc)]
    }

    #[test]
    fn token_validator_trait_is_object_safe_for_both_implementations() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InternalValidator>();
        assert_send_sync::<OidcValidator>();
        assert_send_sync::<Box<dyn TokenValidator>>();
    }
}
