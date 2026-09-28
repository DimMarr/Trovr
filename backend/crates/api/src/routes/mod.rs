use axum::Router;
use axum::routing::get;

use crate::AppState;

pub(crate) mod auth;
pub(crate) mod health;

/// Routes served under `/api/v1`.
pub(crate) fn api() -> Router<AppState> {
    Router::new().route("/me", get(auth::me))
}
