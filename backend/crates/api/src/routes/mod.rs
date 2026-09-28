use axum::Router;
use axum::routing::{get, post};

use crate::AppState;

pub(crate) mod auth;
pub(crate) mod health;

/// Routes served under `/api/v1`.
pub(crate) fn api() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/me", get(auth::me))
}
