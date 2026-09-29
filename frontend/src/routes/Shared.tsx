import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router'

import { files } from '@/api/files'
import { shares } from '@/api/shares'
import type { Node } from '@/api/types'
import { NodeTable } from '@/components/NodeTable'
import { PageHeader } from '@/components/PageHeader'
import { TableSkeleton } from '@/components/TableSkeleton'
import { useDownload } from '@/hooks/download'
import { errorMessage } from '@/lib/errors'
import { strings } from '@/strings'

/** The nodes other people shared with me directly. */
export function Shared() {
  const shared = useQuery({ queryKey: ['shared'], queryFn: shares.sharedWithMe })
  const navigate = useNavigate()
  const download = useDownload((node: Node) => files.download(node.id))

  function open(node: Node) {
    if (node.type === 'folder') navigate(`/folders/${node.id}`)
    else download.mutate(node)
  }

  return (
    <>
      <PageHeader title={strings.sharedWithMe} />
      {shared.isPending ? (
        <TableSkeleton />
      ) : shared.isError ? (
        <p role="alert" className="text-destructive">
          {errorMessage(shared.error)}
        </p>
      ) : shared.data.length === 0 ? (
        <p className="text-muted-foreground py-16 text-center">{strings.nothingShared}</p>
      ) : (
        <NodeTable nodes={shared.data} onOpen={open} />
      )}
    </>
  )
}
