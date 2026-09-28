use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::validation::{validate_content, validate_mime_type, validate_name};
use crate::{MetadataError, NewFile, Node, NodeType};

/// Postgres-backed access to the node tree: folders, files, versions, trash.
///
/// `NodeStore` performs no authorization; callers decide who may act on
/// which node before calling it.
#[derive(Debug, Clone)]
pub struct NodeStore {
    pub(crate) pool: PgPool,
}

impl NodeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_folder(
        &self,
        owner_id: Uuid,
        parent_id: Option<Uuid>,
        name: &str,
    ) -> Result<Node, MetadataError> {
        validate_name(name)?;
        if let Some(parent_id) = parent_id {
            fetch_live_folder(&self.pool, parent_id).await?;
        }

        let node = sqlx::query_as::<_, Node>(
            "INSERT INTO nodes (parent_id, type, name, owner_id) VALUES ($1, 'folder', $2, $3) RETURNING *",
        )
        .bind(parent_id)
        .bind(name)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(node)
    }

    pub async fn create_file(&self, new_file: NewFile) -> Result<Node, MetadataError> {
        validate_name(&new_file.name)?;
        validate_mime_type(&new_file.mime_type)?;
        validate_content(&new_file.content)?;

        let mut tx = self.pool.begin().await?;

        if let Some(parent_id) = new_file.parent_id {
            fetch_live_folder(&mut *tx, parent_id).await?;
        }

        // Spec caveat #2: a file node must never be committed without its
        // first version, so the node, the version and the pointer between
        // them are written in one transaction.
        let node_id: Uuid = sqlx::query_scalar(
            "INSERT INTO nodes (parent_id, type, name, owner_id, size_bytes, mime_type) \
             VALUES ($1, 'file', $2, $3, $4, $5) RETURNING id",
        )
        .bind(new_file.parent_id)
        .bind(&new_file.name)
        .bind(new_file.owner_id)
        .bind(new_file.content.size_bytes)
        .bind(&new_file.mime_type)
        .fetch_one(&mut *tx)
        .await?;

        let version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO file_versions (node_id, version_number, storage_key, size_bytes, checksum_sha256, created_by) \
             VALUES ($1, 1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(node_id)
        .bind(&new_file.content.storage_key)
        .bind(new_file.content.size_bytes)
        .bind(&new_file.content.checksum_sha256)
        .bind(new_file.owner_id)
        .fetch_one(&mut *tx)
        .await?;

        let node = sqlx::query_as::<_, Node>(
            "UPDATE nodes SET current_version_id = $1 WHERE id = $2 RETURNING *",
        )
        .bind(version_id)
        .bind(node_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(node)
    }

    pub async fn get_node(&self, node_id: Uuid) -> Result<Node, MetadataError> {
        fetch_live_node(&self.pool, node_id).await
    }

    /// Returns the chain of nodes from the owner's root down to `node_id`
    /// (inclusive), e.g. for breadcrumbs.
    pub async fn get_path(&self, node_id: Uuid) -> Result<Vec<Node>, MetadataError> {
        let path = sqlx::query_as::<_, Node>(
            "WITH RECURSIVE ancestry AS (
                 SELECT n.*, 0 AS depth FROM nodes n WHERE n.id = $1
                 UNION ALL
                 SELECT p.*, a.depth + 1 FROM nodes p JOIN ancestry a ON p.id = a.parent_id
             )
             SELECT * FROM ancestry ORDER BY depth DESC",
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await?;

        if path.is_empty() || path.iter().any(|node| node.trashed_at.is_some()) {
            return Err(MetadataError::NotFound);
        }

        Ok(path)
    }

    /// Lists the owner's live root-level nodes, folders first, then by name.
    pub async fn list_root(&self, owner_id: Uuid) -> Result<Vec<Node>, MetadataError> {
        let nodes = sqlx::query_as::<_, Node>(
            "SELECT * FROM nodes \
             WHERE parent_id IS NULL AND owner_id = $1 AND trashed_at IS NULL \
             ORDER BY type, name",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(nodes)
    }

    /// Lists a live folder's live children, folders first, then by name.
    pub async fn list_children(&self, folder_id: Uuid) -> Result<Vec<Node>, MetadataError> {
        fetch_live_folder(&self.pool, folder_id).await?;

        let nodes = sqlx::query_as::<_, Node>(
            "SELECT * FROM nodes WHERE parent_id = $1 AND trashed_at IS NULL ORDER BY type, name",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(nodes)
    }
}

/// Loads a node that is neither trashed itself nor inside a trashed folder.
pub(crate) async fn fetch_live_node<'c>(
    executor: impl PgExecutor<'c>,
    node_id: Uuid,
) -> Result<Node, MetadataError> {
    sqlx::query_as::<_, Node>(
        "WITH RECURSIVE ancestry AS (
             SELECT id, parent_id, trashed_at FROM nodes WHERE id = $1
             UNION ALL
             SELECT p.id, p.parent_id, p.trashed_at FROM nodes p JOIN ancestry a ON p.id = a.parent_id
         )
         SELECT * FROM nodes
         WHERE id = $1 AND NOT EXISTS (SELECT 1 FROM ancestry WHERE trashed_at IS NOT NULL)",
    )
    .bind(node_id)
    .fetch_optional(executor)
    .await?
    .ok_or(MetadataError::NotFound)
}

pub(crate) async fn fetch_live_folder<'c>(
    executor: impl PgExecutor<'c>,
    folder_id: Uuid,
) -> Result<Node, MetadataError> {
    let node = fetch_live_node(executor, folder_id).await?;
    if node.node_type != NodeType::Folder {
        return Err(MetadataError::NotAFolder);
    }
    Ok(node)
}
