use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "node_type", rename_all = "lowercase")]
pub enum NodeType {
    Folder,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Node {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    #[sqlx(rename = "type")]
    pub node_type: NodeType,
    pub name: String,
    pub owner_id: Uuid,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub current_version_id: Option<Uuid>,
    pub trashed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct FileVersion {
    pub id: Uuid,
    pub node_id: Uuid,
    pub version_number: i32,
    pub storage_key: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// An object already uploaded to the storage backend, to be recorded as a
/// file version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewContent {
    pub storage_key: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFile {
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub mime_type: String,
    pub content: NewContent,
}
