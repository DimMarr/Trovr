import { RotateCcwIcon, Trash2Icon } from 'lucide-react'
import { useState } from 'react'

import type { Node } from '@/api/types'
import { NodeIcon } from '@/components/NodeIcon'
import { PageHeader } from '@/components/PageHeader'
import { TableSkeleton } from '@/components/TableSkeleton'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { usePurge, useRestore, useTrashList } from '@/hooks/trash'
import { errorMessage } from '@/lib/errors'
import { formatDate } from '@/lib/format'
import { sortNodes } from '@/lib/nodes'
import { strings } from '@/strings'

/** My trashed nodes: restore them or delete them forever. */
export function Trash() {
  const list = useTrashList()
  const restore = useRestore()
  const purge = usePurge()
  const [purging, setPurging] = useState<Node | null>(null)

  return (
    <>
      <PageHeader title={strings.trash} />
      {list.isPending ? (
        <TableSkeleton />
      ) : list.isError ? (
        <p role="alert" className="text-destructive">
          {errorMessage(list.error)}
        </p>
      ) : list.data.length === 0 ? (
        <p className="text-muted-foreground py-16 text-center">{strings.trashView.empty}</p>
      ) : (
        <ul className="divide-y rounded-lg border">
          {sortNodes(list.data).map((node) => (
            <li key={node.id} className="flex items-center gap-3 px-3 py-2">
              <NodeIcon node={node} />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm">{node.name}</p>
                {node.trashed_at && (
                  <p className="text-muted-foreground text-xs">
                    {strings.trashView.trashedOn(formatDate(node.trashed_at))}
                  </p>
                )}
              </div>
              <Button
                variant="ghost"
                size="sm"
                aria-label={strings.trashView.restoreLabel(node.name)}
                disabled={restore.isPending}
                onClick={() => restore.mutate(node)}
              >
                <RotateCcwIcon />
                <span className="hidden sm:inline">{strings.trashView.restore}</span>
              </Button>
              <Button
                variant="ghost"
                size="icon"
                aria-label={strings.trashView.purgeLabel(node.name)}
                onClick={() => setPurging(node)}
              >
                <Trash2Icon />
              </Button>
            </li>
          ))}
        </ul>
      )}
      <AlertDialog open={purging !== null} onOpenChange={(open) => !open && setPurging(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{strings.trashView.purgeTitle(purging?.name ?? '')}</AlertDialogTitle>
            <AlertDialogDescription>{strings.trashView.purgeDescription}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{strings.cancel}</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => purging && purge.mutate(purging)}
            >
              {strings.trashView.purge}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}
