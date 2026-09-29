//! Read-only access through a public link, without an account. Every route
//! re-resolves the token, so revoking or expiring a link cuts access at once,
//! and only reaches the link's node and its live descendants.

use axum::Json;
use axum::extract::{Path, State};
use trovr_metadata::Node;
use uuid::Uuid;

use crate::dto::{NodeResponse, PresignedResponse, node_list};
use crate::routes::files::presign_download;
use crate::{ApiError, AppState};

/// Loads a live node reachable through the link; anything else is missing.
async fn linked_node(state: &AppState, token: &str, node_id: Uuid) -> Result<Node, ApiError> {
    let link = state.nodes.resolve_link(token).await?;
    if !state.nodes.is_within(node_id, link.node_id).await? {
        return Err(ApiError::not_found());
    }
    Ok(state.nodes.get_node(node_id).await?)
}

/// The node the link points to.
pub(crate) async fn root(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<NodeResponse>, ApiError> {
    let link = state.nodes.resolve_link(&token).await?;
    let node = state.nodes.get_node(link.node_id).await?;
    Ok(Json(node.into()))
}

pub(crate) async fn list_children(
    State(state): State<AppState>,
    Path((token, node_id)): Path<(String, Uuid)>,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    linked_node(&state, &token, node_id).await?;
    let nodes = state.nodes.list_children(node_id).await?;
    Ok(Json(node_list(nodes)))
}

/// Presigns a GET for the current version of a file under the link.
pub(crate) async fn download(
    State(state): State<AppState>,
    Path((token, node_id)): Path<(String, Uuid)>,
) -> Result<Json<PresignedResponse>, ApiError> {
    let node = linked_node(&state, &token, node_id).await?;
    Ok(Json(presign_download(&state, &node, None).await?))
}
