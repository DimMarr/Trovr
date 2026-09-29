import type { Node } from '@/api/types'

/** Folders first, then files, each by name. */
export function sortNodes(nodes: Node[]): Node[] {
  return [...nodes].sort((a, b) =>
    a.type === b.type ? a.name.localeCompare(b.name) : a.type === 'folder' ? -1 : 1,
  )
}
