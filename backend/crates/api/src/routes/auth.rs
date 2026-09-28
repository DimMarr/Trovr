use axum::Json;

use crate::dto::UserResponse;
use crate::extract::CurrentUser;

pub(crate) async fn me(CurrentUser(user): CurrentUser) -> Json<UserResponse> {
    Json(user.into())
}
