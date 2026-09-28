//! Axum routes, handlers, request/response DTOs.

mod dto;
mod error;
mod extract;
mod routes;
mod state;

use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;

pub use error::ApiError;
pub use state::{ApiSettings, AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(routes::health::live))
        .route("/readyz", get(routes::health::ready))
        .nest("/api/v1", routes::api())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
