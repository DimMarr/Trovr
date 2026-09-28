//! Authorization. In this phase a user may act only on the nodes they own;
//! Phase 6 (sharing) extends these checks with `node_permissions`.

use trovr_auth::AuthenticatedUser;
use trovr_metadata::Node;
use uuid::Uuid;

use crate::{ApiError, AppState};

/// Loads a live node the caller may act on. Other users' nodes are reported
/// as missing so their existence does not leak.
pub(crate) async fn owned_node(
    state: &AppState,
    user: &AuthenticatedUser,
    node_id: Uuid,
) -> Result<Node, ApiError> {
    let node = state.nodes.get_node(node_id).await?;
    ensure_owner(&node, user)?;
    Ok(node)
}

pub(crate) fn ensure_owner(node: &Node, user: &AuthenticatedUser) -> Result<(), ApiError> {
    if node.owner_id == user.user_id {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}
