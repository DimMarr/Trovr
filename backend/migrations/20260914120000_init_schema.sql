CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- =========================
-- USERS
-- =========================
CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    issuer        TEXT NOT NULL,       -- 'internal' or the OIDC issuer URL
    subject       TEXT NOT NULL,       -- 'sub' claim, unique within an issuer
    email         TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (issuer, subject)
);

CREATE TABLE local_credentials (
    user_id        UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    password_hash  TEXT NOT NULL,       -- argon2
    totp_secret    TEXT,                -- NULL if 2FA disabled
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- =========================
-- NODES (unified files + folders)
-- =========================
CREATE TYPE node_type AS ENUM ('folder', 'file');

CREATE TABLE nodes (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id           UUID REFERENCES nodes(id) ON DELETE CASCADE,
    type                node_type NOT NULL,
    name                TEXT NOT NULL,
    owner_id            UUID NOT NULL REFERENCES users(id),
    size_bytes          BIGINT NOT NULL DEFAULT 0,  -- always 0 for a folder
    mime_type           TEXT,                       -- NULL for a folder
    current_version_id  UUID,                       -- FK added after file_versions (circular dependency)
    trashed_at          TIMESTAMPTZ,                -- soft delete / trash
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT unique_name_per_parent UNIQUE (parent_id, name),
    CONSTRAINT file_has_mime CHECK (type = 'folder' OR mime_type IS NOT NULL)
);

CREATE INDEX idx_nodes_parent_id ON nodes(parent_id);
CREATE INDEX idx_nodes_owner_id  ON nodes(owner_id);
CREATE INDEX idx_nodes_trashed   ON nodes(trashed_at) WHERE trashed_at IS NOT NULL;

-- =========================
-- FILE VERSIONS
-- =========================
CREATE TABLE file_versions (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id          UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    version_number   INT NOT NULL,
    storage_key      TEXT NOT NULL,      -- object key in the S3 bucket
    size_bytes       BIGINT NOT NULL,
    checksum_sha256  TEXT,
    created_by       UUID NOT NULL REFERENCES users(id),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (node_id, version_number)
);

ALTER TABLE nodes
    ADD CONSTRAINT fk_current_version
    FOREIGN KEY (current_version_id) REFERENCES file_versions(id);

-- =========================
-- PERMISSIONS / SHARING
-- =========================
CREATE TYPE principal_type   AS ENUM ('user', 'group', 'link');
CREATE TYPE permission_role  AS ENUM ('viewer', 'editor', 'owner');

CREATE TABLE node_permissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id         UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    principal_type  principal_type NOT NULL,
    principal_id    UUID,             -- user_id or group_id; NULL if principal_type = 'link'
    role            permission_role NOT NULL,
    share_token     TEXT,             -- public link only
    expires_at      TIMESTAMPTZ,
    granted_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (node_id, principal_type, principal_id)
);

CREATE UNIQUE INDEX idx_share_token
    ON node_permissions(share_token)
    WHERE share_token IS NOT NULL;
