use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use trovr_auth::{AuthenticatedUser, UserProfile};
use trovr_metadata::{FileVersion, Node, NodeType, Role, Share, ShareLink};
use trovr_storage::PresignedRequest;
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

/// Another user, as shown to someone sharing with them.
#[derive(Debug, Serialize)]
pub(crate) struct UserSummary {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

impl From<UserProfile> for UserSummary {
    fn from(profile: UserProfile) -> Self {
        Self {
            id: profile.id,
            email: profile.email,
            display_name: profile.display_name,
        }
    }
}

pub(crate) fn role_name(role: Role) -> &'static str {
    match role {
        Role::Viewer => "viewer",
        Role::Editor => "editor",
        Role::Owner => "owner",
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ShareResponse {
    pub user: UserSummary,
    pub role: &'static str,
    pub created_at: DateTime<Utc>,
}

impl ShareResponse {
    pub(crate) fn new(share: Share, user: UserSummary) -> Self {
        Self {
            user,
            role: role_name(share.role),
            created_at: share.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct LinkResponse {
    pub id: Uuid,
    pub token: String,
    pub role: &'static str,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ShareLink> for LinkResponse {
    fn from(link: ShareLink) -> Self {
        Self {
            id: link.id,
            token: link.token,
            role: role_name(link.role),
            expires_at: link.expires_at,
            created_at: link.created_at,
        }
    }
}

/// Everyone and every link a node is shared with.
#[derive(Debug, Serialize)]
pub(crate) struct SharesResponse {
    pub users: Vec<ShareResponse>,
    pub links: Vec<LinkResponse>,
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

#[derive(Debug, Serialize)]
pub(crate) struct VersionResponse {
    pub id: Uuid,
    pub version_number: i32,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<FileVersion> for VersionResponse {
    fn from(version: FileVersion) -> Self {
        Self {
            id: version.id,
            version_number: version.version_number,
            size_bytes: version.size_bytes,
            checksum_sha256: version.checksum_sha256,
            created_by: version.created_by,
            created_at: version.created_at,
        }
    }
}

/// A request for the browser to send straight to the storage backend, with
/// every listed header.
#[derive(Debug, Serialize)]
pub(crate) struct PresignedResponse {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub expires_at: DateTime<Utc>,
}

impl PresignedResponse {
    pub(crate) fn new(request: PresignedRequest, ttl: Duration) -> Self {
        let ttl = TimeDelta::from_std(ttl).expect("presigned URL lifetimes fit in a TimeDelta");
        Self {
            method: request.method,
            url: request.url,
            headers: request.headers.into_iter().collect(),
            expires_at: Utc::now() + ttl,
        }
    }
}
