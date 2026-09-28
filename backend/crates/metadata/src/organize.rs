use uuid::Uuid;

use crate::store::{fetch_live_folder, fetch_live_node};
use crate::validation::validate_name;
use crate::{MetadataError, Node, NodeStore};

/// Advisory lock key ("trovr_mv" in ASCII) taken by every move. Checking for
/// cycles per move is not enough on its own: moving A into B while B moves
/// into A concurrently would each pass the check and together form a cycle.
const MOVE_LOCK_KEY: i64 = 0x7472_6f76_725f_6d76;

impl NodeStore {
    pub async fn rename(&self, node_id: Uuid, new_name: &str) -> Result<Node, MetadataError> {
        validate_name(new_name)?;
        fetch_live_node(&self.pool, node_id).await?;

        sqlx::query_as::<_, Node>(
            "UPDATE nodes SET name = $1, updated_at = now() WHERE id = $2 RETURNING *",
        )
        .bind(new_name)
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MetadataError::NotFound)
    }

    /// Moves a node under `new_parent_id`, or to its owner's root when `None`.
    pub async fn move_node(
        &self,
        node_id: Uuid,
        new_parent_id: Option<Uuid>,
    ) -> Result<Node, MetadataError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(MOVE_LOCK_KEY)
            .execute(&mut *tx)
            .await?;

        fetch_live_node(&mut *tx, node_id).await?;

        if let Some(new_parent_id) = new_parent_id {
            fetch_live_folder(&mut *tx, new_parent_id).await?;

            let would_cycle: bool = sqlx::query_scalar(
                "WITH RECURSIVE ancestry AS (
                     SELECT id, parent_id FROM nodes WHERE id = $1
                     UNION ALL
                     SELECT p.id, p.parent_id FROM nodes p JOIN ancestry a ON p.id = a.parent_id
                 )
                 SELECT EXISTS (SELECT 1 FROM ancestry WHERE id = $2)",
            )
            .bind(new_parent_id)
            .bind(node_id)
            .fetch_one(&mut *tx)
            .await?;

            if would_cycle {
                return Err(MetadataError::CycleDetected);
            }
        }

        let node = sqlx::query_as::<_, Node>(
            "UPDATE nodes SET parent_id = $1, updated_at = now() WHERE id = $2 RETURNING *",
        )
        .bind(new_parent_id)
        .bind(node_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(node)
    }
}
