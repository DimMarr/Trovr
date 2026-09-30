import { FileIcon, FolderIcon } from 'lucide-react'

import type { Node } from '@/api/types'

export function NodeIcon({ node }: { node: Pick<Node, 'type'> }) {
  const Icon = node.type === 'folder' ? FolderIcon : FileIcon
  return <Icon className="text-muted-foreground size-4 shrink-0" aria-hidden />
}
