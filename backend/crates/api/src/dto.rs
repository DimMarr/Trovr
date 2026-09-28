use chrono::{DateTime, Utc};
use serde::Serialize;
use trovr_auth::AuthenticatedUser;
use trovr_metadata::{Node, NodeType};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub(crate) struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub issuer: String,
}

impl From<AuthenticatedUser> for UserResponse {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            id: user.user_id,
            email: user.email,
            display_name: user.display_name,
            issuer: user.issuer,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NodeKind {
    Folder,
    File,
}

#[derive(Debug, Serialize)]
pub(crate) struct NodeResponse {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    #[serde(rename = "type")]
    pub kind: NodeKind,
    pub name: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub trashed_at: Option<DateTime<Utc>>,
}

impl From<Node> for NodeResponse {
    fn from(node: Node) -> Self {
        Self {
            id: node.id,
            parent_id: node.parent_id,
            kind: match node.node_type {
                NodeType::Folder => NodeKind::Folder,
                NodeType::File => NodeKind::File,
            },
            name: node.name,
            size_bytes: node.size_bytes,
            mime_type: node.mime_type,
            created_at: node.created_at,
            updated_at: node.updated_at,
            trashed_at: node.trashed_at,
        }
    }
}

pub(crate) fn node_list(nodes: Vec<Node>) -> Vec<NodeResponse> {
    nodes.into_iter().map(NodeResponse::from).collect()
}
