import { DownloadIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link, useLocation, useNavigate, useParams } from 'react-router'

import { ApiError } from '@/api/client'
import { publicLinks } from '@/api/public'
import type { Node } from '@/api/types'
import { Breadcrumb, type Crumb } from '@/components/Breadcrumb'
import { NodeIcon } from '@/components/NodeIcon'
import { NodeTable } from '@/components/NodeTable'
import { PageHeader } from '@/components/PageHeader'
import { TableSkeleton } from '@/components/TableSkeleton'
import { Button } from '@/components/ui/button'
import { useDownload } from '@/hooks/download'
import { usePublicChildren, usePublicRoot } from '@/hooks/public'
import { errorMessage } from '@/lib/errors'
import { formatBytes } from '@/lib/format'
import { strings } from '@/strings'

/** The folders opened since the link's root, carried in the router state. */
interface Trail {
  trail?: { id: string; name: string }[]
}

const isNotFound = (error: unknown) => error instanceof ApiError && error.status === 404

function PublicLayout({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-svh flex-col">
      <header className="border-b px-4 py-3 md:px-6">
        <p className="text-lg font-semibold">{strings.appName}</p>
      </header>
      <main className="min-w-0 flex-1 p-4 md:p-6">{children}</main>
    </div>
  )
}

/** `/s/:token[/:folderId]`: read-only browsing through a public link, without an account. */
export function PublicLink() {
  const { token = '', folderId = null } = useParams()
  const location = useLocation()
  const navigate = useNavigate()
  const root = usePublicRoot(token)
  const currentId = folderId ?? (root.data?.type === 'folder' ? root.data.id : null)
  const children = usePublicChildren(token, currentId)
  const download = useDownload((node: Node) => publicLinks.download(token, node.id))

  if (root.isPending) return <PublicLayout>{<TableSkeleton />}</PublicLayout>
  if (root.isError) {
    return (
      <PublicLayout>
        <p className="text-muted-foreground py-16 text-center">
          {isNotFound(root.error) ? strings.public.dead : errorMessage(root.error)}
        </p>
      </PublicLayout>
    )
  }

  const link = root.data
  if (link.type === 'file') {
    return (
      <PublicLayout>
        <div className="mx-auto flex max-w-md flex-col items-center gap-4 py-16 text-center">
          <NodeIcon node={link} />
          <h1 className="text-xl font-semibold break-all">{link.name}</h1>
          <p className="text-muted-foreground text-sm">{formatBytes(link.size_bytes)}</p>
          <Button onClick={() => download.mutate(link)} disabled={download.isPending}>
            <DownloadIcon />
            {strings.download}
          </Button>
        </div>
      </PublicLayout>
    )
  }

  const trail = (location.state as Trail | null)?.trail ?? []
  const known = folderId === null || trail.at(-1)?.id === folderId
  const rootCrumb: Crumb = { label: link.name, to: folderId === null ? undefined : `/s/${token}` }
  const crumbs: Crumb[] =
    folderId === null
      ? [rootCrumb]
      : known
        ? [
            rootCrumb,
            ...trail.map((step, index) => ({
              label: step.name,
              to: index === trail.length - 1 ? undefined : `/s/${token}/${step.id}`,
            })),
          ]
        : // Reloaded inside a subfolder: the public API has no way to name it.
          [rootCrumb, { label: '…' }]
  const title = folderId === null ? link.name : known ? trail.at(-1)!.name : strings.public.folder

  function open(node: Node) {
    if (node.type === 'file') {
      download.mutate(node)
      return
    }
    const steps = known ? trail : []
    navigate(`/s/${token}/${node.id}`, {
      state: { trail: [...steps, { id: node.id, name: node.name }] } satisfies Trail,
    })
  }

  return (
    <PublicLayout>
      <PageHeader title={title} breadcrumb={folderId !== null && <Breadcrumb crumbs={crumbs} />} />
      {children.isPending ? (
        <TableSkeleton />
      ) : children.isError ? (
        isNotFound(children.error) ? (
          <div className="flex flex-col items-center gap-4 py-16 text-center">
            <p className="text-muted-foreground">{strings.public.outside}</p>
            <Button asChild variant="outline">
              <Link to={`/s/${token}`}>{strings.public.backToRoot}</Link>
            </Button>
          </div>
        ) : (
          <p role="alert" className="text-destructive">
            {errorMessage(children.error)}
          </p>
        )
      ) : children.data.length === 0 ? (
        <p className="text-muted-foreground py-16 text-center">{strings.emptyFolder}</p>
      ) : (
        <NodeTable nodes={children.data} onOpen={open} />
      )}
    </PublicLayout>
  )
}
