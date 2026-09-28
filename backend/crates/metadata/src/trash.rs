use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::store::fetch_live_node;
use crate::{MetadataError, Node, NodeStore};

impl NodeStore {
    /// Moves a live node (and implicitly its whole subtree) to the trash.
    ///
    /// Only the subtree root is marked: descendants disappear because every
    /// live-node lookup walks up through their ancestors.
    pub async fn trash(&self, node_id: Uuid) -> Result<Node, MetadataError> {
        fetch_live_node(&self.pool, node_id).await?;

        sqlx::query_as::<_, Node>(
            "UPDATE nodes SET trashed_at = now() WHERE id = $1 AND trashed_at IS NULL RETURNING *",
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MetadataError::NotFound)
    }

    pub async fn restore(&self, node_id: Uuid) -> Result<Node, MetadataError> {
        let mut tx = self.pool.begin().await?;

        let node = sqlx::query_as::<_, Node>("SELECT * FROM nodes WHERE id = $1 FOR UPDATE")
            .bind(node_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(MetadataError::NotFound)?;

        if node.trashed_at.is_none() {
            return Err(MetadataError::NotTrashed);
        }

        if let Some(parent_id) = node.parent_id {
            match fetch_live_node(&mut *tx, parent_id).await {
                Ok(_) => {}
                Err(MetadataError::NotFound) => return Err(MetadataError::ParentTrashed),
                Err(err) => return Err(err),
            }
        }

        let restored = sqlx::query_as::<_, Node>(
            "UPDATE nodes SET trashed_at = NULL WHERE id = $1 RETURNING *",
        )
        .bind(node_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(restored)
    }

    /// Lists the owner's trashed subtree roots, most recently trashed first.
    pub async fn list_trash(&self, owner_id: Uuid) -> Result<Vec<Node>, MetadataError> {
        let nodes = sqlx::query_as::<_, Node>(
            "SELECT * FROM nodes WHERE owner_id = $1 AND trashed_at IS NOT NULL \
             ORDER BY trashed_at DESC, name",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(nodes)
    }

    /// Permanently deletes a trashed node and its whole subtree.
    ///
    /// Returns the storage keys of every deleted file version so the caller
    /// can remove the objects from the storage backend.
    pub async fn purge(&self, node_id: Uuid) -> Result<Vec<String>, MetadataError> {
        let mut tx = self.pool.begin().await?;

        let trashed_at: Option<Option<DateTime<Utc>>> =
            sqlx::query_scalar("SELECT trashed_at FROM nodes WHERE id = $1 FOR UPDATE")
                .bind(node_id)
                .fetch_optional(&mut *tx)
                .await?;

        match trashed_at {
            None => return Err(MetadataError::NotFound),
            Some(None) => return Err(MetadataError::NotTrashed),
            Some(Some(_)) => {}
        }

        let storage_keys: Vec<String> = sqlx::query_scalar(
            "WITH RECURSIVE subtree AS (
                 SELECT id FROM nodes WHERE id = $1
                 UNION ALL
                 SELECT c.id FROM nodes c JOIN subtree s ON c.parent_id = s.id
             )
             SELECT v.storage_key FROM file_versions v JOIN subtree s ON v.node_id = s.id
             ORDER BY v.storage_key",
        )
        .bind(node_id)
        .fetch_all(&mut *tx)
        .await?;

        // ON DELETE CASCADE removes descendants and their versions.
        sqlx::query("DELETE FROM nodes WHERE id = $1")
            .bind(node_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(storage_keys)
    }
}
