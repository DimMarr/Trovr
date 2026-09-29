import { MoreHorizontalIcon } from 'lucide-react'
import { Fragment } from 'react'

import type { Node, Role } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { type Action, can } from '@/lib/permissions'
import { strings } from '@/strings'

export type NodeCommand = 'download' | 'versions' | 'rename' | 'move' | 'trash'

const items: {
  command: NodeCommand
  action: Action
  label: string
  filesOnly?: boolean
  destructive?: boolean
}[] = [
  { command: 'download', action: 'download', label: strings.download, filesOnly: true },
  { command: 'versions', action: 'versions', label: strings.versions.menu, filesOnly: true },
  { command: 'rename', action: 'rename', label: strings.rename },
  { command: 'move', action: 'move', label: strings.move },
  { command: 'trash', action: 'trash', label: strings.moveToTrash, destructive: true },
]

/** The per-row menu, showing only what the caller's role allows. */
export function NodeActions({
  node,
  role,
  onCommand,
}: {
  node: Node
  role: Role | undefined
  onCommand: (command: NodeCommand, node: Node) => void
}) {
  const allowed = items.filter(
    (item) => can(role, item.action) && (!item.filesOnly || node.type === 'file'),
  )
  if (allowed.length === 0) return null

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={strings.actionsFor(node.name)}>
          <MoreHorizontalIcon />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {allowed.map((item) => (
          <Fragment key={item.command}>
            {item.destructive && <DropdownMenuSeparator />}
            <DropdownMenuItem
              variant={item.destructive ? 'destructive' : 'default'}
              onSelect={() => onCommand(item.command, node)}
            >
              {item.label}
            </DropdownMenuItem>
          </Fragment>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
