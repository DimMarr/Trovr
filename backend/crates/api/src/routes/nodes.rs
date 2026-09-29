use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use trovr_metadata::Role;
use uuid::Uuid;

use crate::access::authorize;
use crate::dto::{NodeResponse, node_list, role_name};
use crate::extract::CurrentUser;
use crate::{ApiError, AppState};

#[derive(Debug, Serialize)]
pub(crate) struct NodeWithRole {
    #[serde(flatten)]
    node: NodeResponse,
    /// The caller's access: `viewer`, `editor` or `owner`.
    role: &'static str,
}

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
        authorize(&state, &user, parent_id, Role::Editor).await?;
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
) -> Result<Json<NodeWithRole>, ApiError> {
    let (node, role) = authorize(&state, &user, node_id, Role::Viewer).await?;
    Ok(Json(NodeWithRole {
        node: node.into(),
        role: role_name(role),
    }))
}

pub(crate) async fn list_children(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    authorize(&state, &user, node_id, Role::Viewer).await?;
    let nodes = state.nodes.list_children(node_id).await?;
    Ok(Json(node_list(nodes)))
}

pub(crate) async fn get_path(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    authorize(&state, &user, node_id, Role::Viewer).await?;
    let mut nodes = state.nodes.get_path(node_id).await?;

    // Access is inherited downwards, so the visible part of the path is a
    // suffix: drop the ancestors above the first node the caller can see.
    let mut hidden = 0;
    for node in &nodes {
        if state
            .nodes
            .effective_role(node.id, user.user_id)
            .await?
            .is_some()
        {
            break;
        }
        hidden += 1;
    }
    nodes.drain(..hidden);
    Ok(Json(node_list(nodes)))
}

pub(crate) async fn rename(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
    Json(body): Json<RenameRequest>,
) -> Result<Json<NodeResponse>, ApiError> {
    authorize(&state, &user, node_id, Role::Editor).await?;
    let node = state.nodes.rename(node_id, &body.name).await?;
    Ok(Json(node.into()))
}

pub(crate) async fn move_node(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
    Json(body): Json<MoveRequest>,
) -> Result<Json<NodeResponse>, ApiError> {
    let (node, _) = authorize(&state, &user, node_id, Role::Editor).await?;
    match body.parent_id {
        Some(parent_id) => {
            authorize(&state, &user, parent_id, Role::Editor).await?;
            // Never move a node somewhere its owner cannot reach.
            let owner_role = state.nodes.effective_role(parent_id, node.owner_id).await?;
            if owner_role < Some(Role::Editor) {
                return Err(ApiError::forbidden());
            }
        }
        // The root a node lands in is its owner's.
        None if node.owner_id != user.user_id => return Err(ApiError::forbidden()),
        None => {}
    }
    let node = state.nodes.move_node(node_id, body.parent_id).await?;
    Ok(Json(node.into()))
}
