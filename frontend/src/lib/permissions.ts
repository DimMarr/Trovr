import type { Role } from '@/api/types'

export type Action =
  | 'download'
  | 'versions'
  | 'createFolder'
  | 'upload'
  | 'rename'
  | 'move'
  | 'trash'
  | 'addVersion'
  | 'share'

const minimumRole: Record<Action, Role> = {
  download: 'viewer',
  versions: 'viewer',
  createFolder: 'editor',
  upload: 'editor',
  rename: 'editor',
  move: 'editor',
  trash: 'editor',
  addVersion: 'editor',
  share: 'owner',
}

const rank: Record<Role, number> = { viewer: 0, editor: 1, owner: 2 }

/** Whether a role allows an action; the API stays the authority. */
export function can(role: Role | undefined, action: Action): boolean {
  return role !== undefined && rank[role] >= rank[minimumRole[action]]
}
