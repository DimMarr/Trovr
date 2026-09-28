use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::access::owned_node;
use crate::dto::{NodeResponse, node_list};
use crate::extract::CurrentUser;
use crate::{ApiError, AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct CreateFolderRequest {
    #[serde(default)]
    parent_id: Option<Uuid>,
    name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RenameRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MoveRequest {
    /// The destination folder; `null` (or absent) moves to the root.
    #[serde(default)]
    parent_id: Option<Uuid>,
}

pub(crate) async fn list_root(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    let nodes = state.nodes.list_root(user.user_id).await?;
    Ok(Json(node_list(nodes)))
}

pub(crate) async fn create_folder(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<CreateFolderRequest>,
) -> Result<(StatusCode, Json<NodeResponse>), ApiError> {
    if let Some(parent_id) = body.parent_id {
        owned_node(&state, &user, parent_id).await?;
    }
    let folder = state
        .nodes
        .create_folder(user.user_id, body.parent_id, &body.name)
        .await?;
    Ok((StatusCode::CREATED, Json(folder.into())))
}

pub(crate) async fn get_node(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<NodeResponse>, ApiError> {
    let node = owned_node(&state, &user, node_id).await?;
    Ok(Json(node.into()))
}

pub(crate) async fn list_children(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    owned_node(&state, &user, node_id).await?;
    let nodes = state.nodes.list_children(node_id).await?;
    Ok(Json(node_list(nodes)))
}

pub(crate) async fn get_path(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    owned_node(&state, &user, node_id).await?;
    let nodes = state.nodes.get_path(node_id).await?;
    Ok(Json(node_list(nodes)))
}

pub(crate) async fn rename(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
    Json(body): Json<RenameRequest>,
) -> Result<Json<NodeResponse>, ApiError> {
    owned_node(&state, &user, node_id).await?;
    let node = state.nodes.rename(node_id, &body.name).await?;
    Ok(Json(node.into()))
}

pub(crate) async fn move_node(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
    Json(body): Json<MoveRequest>,
) -> Result<Json<NodeResponse>, ApiError> {
    owned_node(&state, &user, node_id).await?;
    if let Some(parent_id) = body.parent_id {
        owned_node(&state, &user, parent_id).await?;
    }
    let node = state.nodes.move_node(node_id, body.parent_id).await?;
    Ok(Json(node.into()))
}
