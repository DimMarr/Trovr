import { ChevronRightIcon, FolderIcon } from 'lucide-react'
import { useState } from 'react'

import type { Node } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useMove } from '@/hooks/mutations'
import { useChildren, usePath } from '@/hooks/nodes'
import { sortNodes } from '@/lib/nodes'
import { strings } from '@/strings'

/**
 * Picks a destination folder by browsing. "My files" (the root) is only offered to the
 * node's owner: a node always lands in its owner's tree.
 */
export function MoveDialog({
  node,
  canMoveToRoot,
  onOpenChange,
}: {
  node: Node | null
  canMoveToRoot: boolean
  onOpenChange: (open: boolean) => void
}) {
  return (
    <Dialog open={node !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        {node && (
          <MovePicker
            node={node}
            canMoveToRoot={canMoveToRoot}
            onDone={() => onOpenChange(false)}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

function MovePicker({
  node,
  canMoveToRoot,
  onDone,
}: {
  node: Node
  canMoveToRoot: boolean
  onDone: () => void
}) {
  const [current, setCurrent] = useState<string | null>(canMoveToRoot ? null : node.parent_id)
  const children = useChildren(current)
  const path = usePath(current)
  const move = useMove()

  const folders = sortNodes(children.data ?? []).filter(
    (child) => child.type === 'folder' && child.id !== node.id,
  )
  const sameFolder = current === node.parent_id

  return (
    <>
      <DialogHeader>
        <DialogTitle>{strings.moveTitle(node.name)}</DialogTitle>
        <DialogDescription>{strings.moveDescription}</DialogDescription>
      </DialogHeader>
      <nav
        aria-label="Destination"
        className="text-muted-foreground flex flex-wrap items-center gap-1 text-sm"
      >
        {canMoveToRoot && (
          <button type="button" className="hover:underline" onClick={() => setCurrent(null)}>
            {strings.myFiles}
          </button>
        )}
        {current !== null &&
          (path.data ?? []).map((crumb, index) => (
            <span key={crumb.id} className="flex items-center gap-1">
              {(canMoveToRoot || index > 0) && (
                <ChevronRightIcon className="size-3.5" aria-hidden />
              )}
              <button
                type="button"
                className="hover:underline"
                onClick={() => setCurrent(crumb.id)}
              >
                {crumb.name}
              </button>
            </span>
          ))}
      </nav>
      <ul className="max-h-64 min-h-24 overflow-y-auto rounded-md border">
        {folders.map((folder) => (
          <li key={folder.id}>
            <button
              type="button"
              className="hover:bg-accent flex w-full items-center gap-2 px-3 py-2 text-left text-sm"
              onClick={() => setCurrent(folder.id)}
            >
              <FolderIcon className="text-muted-foreground size-4" aria-hidden />
              {folder.name}
            </button>
          </li>
        ))}
        {children.isSuccess && folders.length === 0 && (
          <li className="text-muted-foreground px-3 py-2 text-sm">{strings.noSubfolders}</li>
        )}
      </ul>
      <DialogFooter>
        <Button
          disabled={sameFolder || move.isPending}
          onClick={() => move.mutate({ node, parentId: current }, { onSuccess: onDone })}
        >
          {strings.moveHere}
        </Button>
      </DialogFooter>
    </>
  )
}
