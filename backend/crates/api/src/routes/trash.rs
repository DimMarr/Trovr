use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use trovr_metadata::Role;
use uuid::Uuid;

use crate::access::{authorize, authorize_any_state};
use crate::dto::{NodeResponse, node_list};
use crate::extract::CurrentUser;
use crate::{ApiError, AppState};

pub(crate) async fn trash(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<NodeResponse>, ApiError> {
    authorize(&state, &user, node_id, Role::Editor).await?;
    let node = state.nodes.trash(node_id).await?;
    Ok(Json(node.into()))
}

pub(crate) async fn restore(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<NodeResponse>, ApiError> {
    authorize_any_state(&state, &user, node_id, Role::Owner).await?;
    let node = state.nodes.restore(node_id).await?;
    Ok(Json(node.into()))
}

pub(crate) async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    let nodes = state.nodes.list_trash(user.user_id).await?;
    Ok(Json(node_list(nodes)))
}

/// Permanently deletes a trashed subtree, then its stored objects. The
/// database is the source of truth: if storage fails after the purge
/// committed, the objects are orphaned and logged, not reported.
pub(crate) async fn purge(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    authorize_any_state(&state, &user, node_id, Role::Owner).await?;

    let storage_keys = state.nodes.purge(node_id).await?;
    if let Err(err) = state.storage.delete(&storage_keys).await {
        tracing::warn!(
            error = %err,
            orphaned_objects = storage_keys.len(),
            "failed to delete purged objects from storage"
        );
    }

    Ok(StatusCode::NO_CONTENT)
}
