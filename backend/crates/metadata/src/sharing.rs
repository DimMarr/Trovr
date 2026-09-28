use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{MetadataError, Node, NodeStore};

/// Access level on a node, ordered `Viewer < Editor < Owner` like the
/// Postgres `permission_role` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, sqlx::Type)]
#[sqlx(type_name = "permission_role", rename_all = "lowercase")]
pub enum Role {
    Viewer,
    Editor,
    Owner,
}

/// A node shared with one user; inherited by the node's whole subtree.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Share {
    pub id: Uuid,
    pub node_id: Uuid,
    #[sqlx(rename = "principal_id")]
    pub user_id: Uuid,
    pub role: Role,
    pub granted_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl NodeStore {
    /// The caller's access to a node, whatever its trash state: `Owner` when
    /// they own it or any ancestor, else the highest live grant on it or any
    /// ancestor (spec caveat #3: permissions are inherited, not copied).
    pub async fn effective_role(
        &self,
        node_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Role>, MetadataError> {
        let (exists, owns, granted): (bool, bool, Option<Role>) = sqlx::query_as(
            "WITH RECURSIVE ancestry AS (
                 SELECT id, parent_id, owner_id FROM nodes WHERE id = $1
                 UNION ALL
                 SELECT p.id, p.parent_id, p.owner_id FROM nodes p JOIN ancestry a ON p.id = a.parent_id
             )
             SELECT
                 EXISTS (SELECT 1 FROM ancestry),
                 EXISTS (SELECT 1 FROM ancestry WHERE owner_id = $2),
                 (SELECT max(np.role) FROM node_permissions np JOIN ancestry a ON np.node_id = a.id
                  WHERE np.principal_type = 'user' AND np.principal_id = $2
                    AND (np.expires_at IS NULL OR np.expires_at > now()))",
        )
        .bind(node_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists {
            return Err(MetadataError::NotFound);
        }
        Ok(if owns { Some(Role::Owner) } else { granted })
    }

    /// Grants `role` on a node to a user, replacing any previous grant.
    pub async fn share_with_user(
        &self,
        node_id: Uuid,
        user_id: Uuid,
        role: Role,
        granted_by: Uuid,
    ) -> Result<Share, MetadataError> {
        let share = sqlx::query_as::<_, Share>(
            "INSERT INTO node_permissions (node_id, principal_type, principal_id, role, granted_by) \
             VALUES ($1, 'user', $2, $3, $4) \
             ON CONFLICT (node_id, principal_type, principal_id) DO UPDATE \
             SET role = EXCLUDED.role, granted_by = EXCLUDED.granted_by, expires_at = NULL \
             RETURNING id, node_id, principal_id, role, granted_by, expires_at, created_at",
        )
        .bind(node_id)
        .bind(user_id)
        .bind(role)
        .bind(granted_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(share)
    }

    pub async fn unshare_with_user(
        &self,
        node_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), MetadataError> {
        let deleted = sqlx::query(
            "DELETE FROM node_permissions \
             WHERE node_id = $1 AND principal_type = 'user' AND principal_id = $2",
        )
        .bind(node_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if deleted == 0 {
            return Err(MetadataError::NotFound);
        }
        Ok(())
    }

    /// Lists the direct user grants on a node, oldest first.
    pub async fn list_user_shares(&self, node_id: Uuid) -> Result<Vec<Share>, MetadataError> {
        let shares = sqlx::query_as::<_, Share>(
            "SELECT id, node_id, principal_id, role, granted_by, expires_at, created_at \
             FROM node_permissions WHERE node_id = $1 AND principal_type = 'user' \
             ORDER BY created_at, id",
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(shares)
    }

    /// Lists the live nodes directly shared with a user (not the ones they
    /// own), by name — the entry points of their "shared with me" view.
    pub async fn list_shared_with(&self, user_id: Uuid) -> Result<Vec<Node>, MetadataError> {
        let nodes = sqlx::query_as::<_, Node>(
            "WITH RECURSIVE shared AS (
                 SELECT n.* FROM nodes n JOIN node_permissions np ON np.node_id = n.id
                 WHERE np.principal_type = 'user' AND np.principal_id = $1
                   AND (np.expires_at IS NULL OR np.expires_at > now())
                   AND n.owner_id <> $1
             ),
             ancestry AS (
                 SELECT s.id AS shared_id, s.id, s.parent_id, s.trashed_at FROM shared s
                 UNION ALL
                 SELECT a.shared_id, p.id, p.parent_id, p.trashed_at
                 FROM nodes p JOIN ancestry a ON p.id = a.parent_id
             )
             SELECT * FROM shared s
             WHERE NOT EXISTS (
                 SELECT 1 FROM ancestry a WHERE a.shared_id = s.id AND a.trashed_at IS NOT NULL
             )
             ORDER BY s.name, s.id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(nodes)
    }
}
