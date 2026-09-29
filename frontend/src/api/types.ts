export type Role = 'viewer' | 'editor' | 'owner'
export type GrantableRole = Exclude<Role, 'owner'>

export interface Node {
  id: string
  parent_id: string | null
  type: 'folder' | 'file'
  name: string
  size_bytes: number
  mime_type: string | null
  created_at: string
  updated_at: string
  trashed_at: string | null
}

export type NodeWithRole = Node & { role: Role }

/** A request the browser sends as-is, straight to object storage. */
export interface Presigned {
  method: string
  url: string
  headers: Record<string, string>
  expires_at: string
}

export interface Upload {
  storage_key: string
  upload: Presigned
}

export interface Version {
  id: string
  version_number: number
  size_bytes: number
  checksum_sha256: string | null
  created_by: string
  created_at: string
}

export interface User {
  id: string
  email: string
  display_name: string
  issuer: string
}

export interface UserSummary {
  id: string
  email: string
  display_name: string
}

export interface Share {
  user: UserSummary
  role: GrantableRole
  created_at: string
}

export interface Link {
  id: string
  token: string
  role: 'viewer'
  expires_at: string | null
  created_at: string
}

export interface Shares {
  users: Share[]
  links: Link[]
}

export interface AuthConfig {
  internal: { enabled: boolean; registration: boolean }
  oidc: { issuer: string; client_id: string } | null
  max_upload_bytes: number
}
