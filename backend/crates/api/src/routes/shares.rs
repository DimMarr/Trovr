use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Deserialize;
use trovr_auth::{find_users_by_email, find_users_by_ids};
use trovr_metadata::Role;
use uuid::Uuid;

use crate::access::authorize;
use crate::dto::{
    LinkResponse, NodeResponse, ShareResponse, SharesResponse, UserSummary, node_list,
};
use crate::extract::CurrentUser;
use crate::{ApiError, AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct ShareRequest {
    email: String,
    role: String,
}

/// Only `viewer` and `editor` can be granted; ownership never moves.
fn grantable_role(role: &str) -> Result<Role, ApiError> {
    match role {
        "viewer" => Ok(Role::Viewer),
        "editor" => Ok(Role::Editor),
        _ => Err(ApiError::invalid_request(
            "role must be \"viewer\" or \"editor\"",
        )),
    }
}

pub(crate) async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<SharesResponse>, ApiError> {
    authorize(&state, &user, node_id, Role::Owner).await?;
    let shares = state.nodes.list_user_shares(node_id).await?;
    let links = state.nodes.list_links(node_id).await?;

    let ids: Vec<Uuid> = shares.iter().map(|share| share.user_id).collect();
    let mut profiles: HashMap<Uuid, UserSummary> = find_users_by_ids(&state.pool, &ids)
        .await?
        .into_iter()
        .map(|profile| (profile.id, profile.into()))
        .collect();
    // A grant always names an existing user (they are never deleted).
    let users = shares
        .into_iter()
        .filter_map(|share| {
            let profile = profiles.remove(&share.user_id)?;
            Some(ShareResponse::new(share, profile))
        })
        .collect();

    Ok(Json(SharesResponse {
        users,
        links: links.into_iter().map(LinkResponse::from).collect(),
    }))
}

/// Grants a role to the user with this email, or changes their role.
pub(crate) async fn share(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
    Json(body): Json<ShareRequest>,
) -> Result<(StatusCode, Json<ShareResponse>), ApiError> {
    authorize(&state, &user, node_id, Role::Owner).await?;
    let role = grantable_role(&body.role)?;

    let mut matches = find_users_by_email(&state.pool, &body.email).await?;
    let grantee = match matches.len() {
        0 => {
            return Err(ApiError::new(
                StatusCode::NOT_FOUND,
                "user_not_found",
                "no user has this email",
            ));
        }
        1 => matches.remove(0),
        _ => {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "ambiguous_user",
                "several accounts share this email",
            ));
        }
    };
    // Covers sharing with oneself, with the node's owner, and with the owner
    // of any folder above it: a grant could only lower what they hold.
    if state.nodes.effective_role(node_id, grantee.id).await? == Some(Role::Owner) {
        return Err(ApiError::invalid_request("this user already owns the node"));
    }

    let share = state
        .nodes
        .share_with_user(node_id, grantee.id, role, user.user_id)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(ShareResponse::new(share, grantee.into())),
    ))
}

pub(crate) async fn unshare(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((node_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    authorize(&state, &user, node_id, Role::Owner).await?;
    state.nodes.unshare_with_user(node_id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The nodes other users shared with the caller directly.
pub(crate) async fn shared_with_me(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<NodeResponse>>, ApiError> {
    let nodes = state.nodes.list_shared_with(user.user_id).await?;
    Ok(Json(node_list(nodes)))
}
