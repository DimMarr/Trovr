import { useNavigate, useParams } from 'react-router'

import { ApiError } from '@/api/client'
import type { Node } from '@/api/types'
import { Breadcrumb, type Crumb } from '@/components/Breadcrumb'
import { NodeTable } from '@/components/NodeTable'
import { NotFound } from '@/components/NotFound'
import { PageHeader } from '@/components/PageHeader'
import { TableSkeleton } from '@/components/TableSkeleton'
import { useChildren, useNode, usePath } from '@/hooks/nodes'
import { errorMessage } from '@/lib/errors'
import { strings } from '@/strings'

function isMissing(error: unknown) {
  return error instanceof ApiError && (error.status === 404 || error.status === 403)
}

/** `/` (my root) and `/folders/:id` (any folder I can see). */
export function Browser() {
  const { id = null } = useParams()
  const navigate = useNavigate()
  const folder = useNode(id)
  const children = useChildren(id)
  const path = usePath(id)

  if (isMissing(folder.error) || isMissing(children.error)) return <NotFound />

  // Access is inherited downwards: only a sharee's view starts somewhere else than their root.
  const inMyTree = id === null || folder.data?.role === 'owner'
  const crumbs: Crumb[] = [
    inMyTree ? { label: strings.myFiles, to: '/' } : { label: strings.sharedWithMe, to: '/shared' },
    ...(path.data ?? []).map((node) => ({
      label: node.name,
      to: node.id === id ? undefined : `/folders/${node.id}`,
    })),
  ]
  if (id === null) crumbs[0] = { label: strings.myFiles }

  function open(node: Node) {
    if (node.type === 'folder') navigate(`/folders/${node.id}`)
  }

  const title = id === null ? strings.myFiles : (folder.data?.name ?? '')

  return (
    <>
      <PageHeader title={title} breadcrumb={id !== null && <Breadcrumb crumbs={crumbs} />} />
      {children.isPending ? (
        <TableSkeleton />
      ) : children.isError ? (
        <p role="alert" className="text-destructive">
          {errorMessage(children.error)}
        </p>
      ) : children.data.length === 0 ? (
        <p className="text-muted-foreground py-16 text-center">{strings.emptyFolder}</p>
      ) : (
        <NodeTable nodes={children.data} onOpen={open} />
      )}
    </>
  )
}
