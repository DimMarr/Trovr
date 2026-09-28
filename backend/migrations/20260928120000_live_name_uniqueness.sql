-- Names only need to be unique among live (non-trashed) siblings, so a
-- trashed node no longer blocks re-creating a node with the same name.
-- Root nodes (parent_id IS NULL) are scoped per owner: the original
-- UNIQUE (parent_id, name) never fired for them because NULLs never collide.
ALTER TABLE nodes DROP CONSTRAINT unique_name_per_parent;

CREATE UNIQUE INDEX unique_live_name_per_parent
    ON nodes (parent_id, name)
    WHERE parent_id IS NOT NULL AND trashed_at IS NULL;

CREATE UNIQUE INDEX unique_live_root_name_per_owner
    ON nodes (owner_id, name)
    WHERE parent_id IS NULL AND trashed_at IS NULL;
