import type { ReactNode } from 'react'

import type { Node } from '@/api/types'
import { NodeIcon } from '@/components/NodeIcon'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { formatBytes, formatDate } from '@/lib/format'
import { sortNodes } from '@/lib/nodes'
import { strings } from '@/strings'

export function NodeTable({
  nodes,
  onOpen,
  actions,
}: {
  nodes: Node[]
  onOpen: (node: Node) => void
  /** Extra controls rendered at the end of each row. */
  actions?: (node: Node) => ReactNode
}) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{strings.columns.name}</TableHead>
          <TableHead className="w-28 text-right">{strings.columns.size}</TableHead>
          <TableHead className="hidden w-48 sm:table-cell">{strings.columns.modified}</TableHead>
          {actions && <TableHead className="w-12" />}
        </TableRow>
      </TableHeader>
      <TableBody>
        {sortNodes(nodes).map((node) => (
          <TableRow key={node.id}>
            <TableCell className="max-w-0">
              <button
                type="button"
                className="flex w-full min-w-0 items-center gap-2 text-left hover:underline"
                onClick={() => onOpen(node)}
              >
                <NodeIcon node={node} />
                <span className="truncate">{node.name}</span>
              </button>
            </TableCell>
            <TableCell className="text-muted-foreground text-right tabular-nums">
              {node.type === 'file' ? formatBytes(node.size_bytes) : '—'}
            </TableCell>
            <TableCell className="text-muted-foreground hidden sm:table-cell">
              {formatDate(node.updated_at)}
            </TableCell>
            {actions && <TableCell className="text-right">{actions(node)}</TableCell>}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
