import { FolderPlusIcon, UploadIcon } from 'lucide-react'
import { useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router'

import { ApiError } from '@/api/client'
import { files } from '@/api/files'
import type { Node } from '@/api/types'
import { Breadcrumb, type Crumb } from '@/components/Breadcrumb'
import { MoveDialog } from '@/components/dialogs/MoveDialog'
import { NameDialog } from '@/components/dialogs/NameDialog'
import { DropZone } from '@/components/DropZone'
import { NodeActions, type NodeCommand } from '@/components/NodeActions'
import { NodeTable } from '@/components/NodeTable'
import { NotFound } from '@/components/NotFound'
import { PageHeader } from '@/components/PageHeader'
import { TableSkeleton } from '@/components/TableSkeleton'
import { Button } from '@/components/ui/button'
import { useDownload } from '@/hooks/download'
import { useCreateFolder, useRename, useTrash } from '@/hooks/mutations'
import { useChildren, useNode, usePath } from '@/hooks/nodes'
import { errorMessage } from '@/lib/errors'
import { can } from '@/lib/permissions'
import { strings } from '@/strings'
import { useUploads } from '@/transfers/context'

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
  const createFolder = useCreateFolder(id)
  const rename = useRename()
  const trash = useTrash()
  const [creating, setCreating] = useState(false)
  const [renaming, setRenaming] = useState<Node | null>(null)
  const [moving, setMoving] = useState<Node | null>(null)
  const uploads = useUploads()
  const picker = useRef<HTMLInputElement>(null)
  const download = useDownload((node: Node) => files.download(node.id))

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
    else download.mutate(node)
  }

  function run(command: NodeCommand, node: Node) {
    if (command === 'rename') setRenaming(node)
    else if (command === 'move') setMoving(node)
    else trash.mutate(node)
  }

  // At the root the caller is the owner; inside a folder, its role is inherited by the children.
  const role = id === null ? 'owner' : folder.data?.role
  const title = id === null ? strings.myFiles : (folder.data?.name ?? '')

  return (
    <>
      <PageHeader
        title={title}
        breadcrumb={id !== null && <Breadcrumb crumbs={crumbs} />}
        actions={
          <>
            {can(role, 'createFolder') && (
              <Button variant="outline" onClick={() => setCreating(true)}>
                <FolderPlusIcon />
                {strings.newFolder}
              </Button>
            )}
            {can(role, 'upload') && (
              <>
                <Button onClick={() => picker.current?.click()}>
                  <UploadIcon />
                  {strings.upload}
                </Button>
                <input
                  ref={picker}
                  type="file"
                  multiple
                  hidden
                  aria-label={strings.uploadFiles}
                  onChange={(event) => {
                    uploads.enqueue(Array.from(event.target.files ?? []), id)
                    event.target.value = ''
                  }}
                />
              </>
            )}
          </>
        }
      />
      <DropZone enabled={can(role, 'upload')} onFiles={(dropped) => uploads.enqueue(dropped, id)}>
        {children.isPending ? (
          <TableSkeleton />
        ) : children.isError ? (
          <p role="alert" className="text-destructive">
            {errorMessage(children.error)}
          </p>
        ) : children.data.length === 0 ? (
          <p className="text-muted-foreground py-16 text-center">{strings.emptyFolder}</p>
        ) : (
          <NodeTable
            nodes={children.data}
            onOpen={open}
            actions={(node) => <NodeActions node={node} role={role} onCommand={run} />}
          />
        )}
      </DropZone>
      <NameDialog
        open={creating}
        onOpenChange={setCreating}
        title={strings.newFolder}
        submitLabel={strings.create}
        onSubmit={(name) => createFolder.mutateAsync(name)}
      />
      <NameDialog
        open={renaming !== null}
        onOpenChange={(open) => !open && setRenaming(null)}
        title={strings.renameTitle(renaming?.name ?? '')}
        submitLabel={strings.rename}
        initialName={renaming?.name}
        onSubmit={(name) => rename.mutateAsync({ node: renaming!, name })}
      />
      <MoveDialog
        node={moving}
        canMoveToRoot={role === 'owner'}
        onOpenChange={(open) => !open && setMoving(null)}
      />
    </>
  )
}
