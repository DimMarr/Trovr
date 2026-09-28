use sqlx::PgExecutor;
use uuid::Uuid;

use crate::store::fetch_live_node;
use crate::validation::validate_content;
use crate::{FileVersion, MetadataError, NewContent, Node, NodeStore, NodeType};

impl NodeStore {
    /// Records newly uploaded content as the file's next, now current, version.
    pub async fn add_version(
        &self,
        node_id: Uuid,
        created_by: Uuid,
        content: NewContent,
    ) -> Result<FileVersion, MetadataError> {
        validate_content(&content)?;

        let mut tx = self.pool.begin().await?;

        // Locking the node row serializes concurrent uploads to the same file,
        // so each one computes a distinct next version number.
        sqlx::query("SELECT 1 FROM nodes WHERE id = $1 FOR UPDATE")
            .bind(node_id)
            .execute(&mut *tx)
            .await?;

        fetch_live_file(&mut *tx, node_id).await?;

        let version = sqlx::query_as::<_, FileVersion>(
            "INSERT INTO file_versions (node_id, version_number, storage_key, size_bytes, checksum_sha256, created_by) \
             SELECT $1, COALESCE(MAX(version_number), 0) + 1, $2, $3, $4, $5 \
             FROM file_versions WHERE node_id = $1 \
             RETURNING *",
        )
        .bind(node_id)
        .bind(&content.storage_key)
        .bind(content.size_bytes)
        .bind(&content.checksum_sha256)
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE nodes SET current_version_id = $1, size_bytes = $2, updated_at = now() WHERE id = $3",
        )
        .bind(version.id)
        .bind(version.size_bytes)
        .bind(node_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(version)
    }

    /// Lists a live file's versions, newest first.
    pub async fn list_versions(&self, node_id: Uuid) -> Result<Vec<FileVersion>, MetadataError> {
        fetch_live_file(&self.pool, node_id).await?;

        let versions = sqlx::query_as::<_, FileVersion>(
            "SELECT * FROM file_versions WHERE node_id = $1 ORDER BY version_number DESC",
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(versions)
    }

    pub async fn current_version(&self, node_id: Uuid) -> Result<FileVersion, MetadataError> {
        fetch_live_file(&self.pool, node_id).await?;

        sqlx::query_as::<_, FileVersion>(
            "SELECT v.* FROM file_versions v JOIN nodes n ON n.current_version_id = v.id WHERE n.id = $1",
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MetadataError::NotFound)
    }
}

async fn fetch_live_file<'c>(
    executor: impl PgExecutor<'c>,
    node_id: Uuid,
) -> Result<Node, MetadataError> {
    let node = fetch_live_node(executor, node_id).await?;
    if node.node_type != NodeType::File {
        return Err(MetadataError::NotAFile);
    }
    Ok(node)
}
