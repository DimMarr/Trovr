use axum::extract::State;
use axum::http::StatusCode;

use crate::{ApiError, AppState};

/// Liveness: the process is up.
pub(crate) async fn live() -> &'static str {
    "ok"
}

/// Readiness: the database answers, so this instance can serve traffic.
pub(crate) async fn ready(State(state): State<AppState>) -> Result<&'static str, ApiError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "readiness check failed");
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "not_ready",
                "database unreachable",
            )
        })?;
    Ok("ok")
}
