use serde::Serialize;
use trovr_auth::AuthenticatedUser;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub(crate) struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub issuer: String,
}

impl From<AuthenticatedUser> for UserResponse {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            id: user.user_id,
            email: user.email,
            display_name: user.display_name,
            issuer: user.issuer,
        }
    }
}
