//! Authorization: a caller's role on a node is inherited from its ancestors
//! (ownership of any ancestor, or a live share on any ancestor).

use trovr_auth::AuthenticatedUser;
use trovr_metadata::{Node, Role};
use uuid::Uuid;

use crate::{ApiError, AppState};

/// Loads a live node the caller holds at least `needed` on, with their role.
/// No access at all is reported as missing so existence does not leak.
pub(crate) async fn authorize(
    state: &AppState,
    user: &AuthenticatedUser,
    node_id: Uuid,
    needed: Role,
) -> Result<(Node, Role), ApiError> {
    let node = state.nodes.get_node(node_id).await?;
    let role = require(
        state.nodes.effective_role(node_id, user.user_id).await?,
        needed,
    )?;
    Ok((node, role))
}

/// Like [`authorize`], for nodes that may be in the trash (restore, purge).
pub(crate) async fn authorize_any_state(
    state: &AppState,
    user: &AuthenticatedUser,
    node_id: Uuid,
    needed: Role,
) -> Result<Node, ApiError> {
    let node = state.nodes.find_node(node_id).await?;
    require(
        state.nodes.effective_role(node_id, user.user_id).await?,
        needed,
    )?;
    Ok(node)
}

/// `None` → 404, too low → 403.
pub(crate) fn require(role: Option<Role>, needed: Role) -> Result<Role, ApiError> {
    match role {
        None => Err(ApiError::not_found()),
        Some(role) if role < needed => Err(ApiError::forbidden()),
        Some(role) => Ok(role),
    }
}

pub(crate) fn role_name(role: Role) -> &'static str {
    match role {
        Role::Viewer => "viewer",
        Role::Editor => "editor",
        Role::Owner => "owner",
    }
}
