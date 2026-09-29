import { DownloadIcon, Loader2Icon, UploadIcon } from 'lucide-react'
import { useRef } from 'react'

import { files } from '@/api/files'
import type { Node, Role, Version } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet'
import { useDownload } from '@/hooks/download'
import { useAddVersion, useVersions } from '@/hooks/versions'
import { formatBytes, formatDate } from '@/lib/format'
import { can } from '@/lib/permissions'
import { strings } from '@/strings'

/** A file's version history: download any version, upload a new one (editors). */
export function VersionsSheet({
  node,
  role,
  onOpenChange,
}: {
  node: Node | null
  role: Role | undefined
  onOpenChange: (open: boolean) => void
}) {
  return (
    <Sheet open={node !== null} onOpenChange={onOpenChange}>
      <SheetContent className="flex flex-col gap-0">
        {node && <VersionsList node={node} role={role} />}
      </SheetContent>
    </Sheet>
  )
}

function VersionsList({ node, role }: { node: Node; role: Role | undefined }) {
  const versions = useVersions(node.id)
  const addVersion = useAddVersion(node)
  const download = useDownload((version: Version) => files.download(node.id, version.id))
  const picker = useRef<HTMLInputElement>(null)

  return (
    <>
      <SheetHeader>
        <SheetTitle className="truncate">{strings.versions.title(node.name)}</SheetTitle>
        <SheetDescription>{strings.versions.description}</SheetDescription>
      </SheetHeader>
      {can(role, 'addVersion') && (
        <div className="px-4 pb-4">
          <Button
            className="w-full"
            disabled={addVersion.isPending}
            onClick={() => picker.current?.click()}
          >
            {addVersion.isPending ? <Loader2Icon className="animate-spin" /> : <UploadIcon />}
            {strings.versions.upload}
          </Button>
          <input
            ref={picker}
            type="file"
            hidden
            aria-label={strings.versions.upload}
            onChange={(event) => {
              const file = event.target.files?.[0]
              if (file) addVersion.mutate(file)
              event.target.value = ''
            }}
          />
        </div>
      )}
      <ul className="flex flex-col overflow-y-auto border-t">
        {(versions.data ?? []).map((version, index) => (
          <li key={version.id} className="flex items-center gap-3 border-b px-4 py-3">
            <div className="min-w-0 flex-1">
              <p className="flex items-center gap-2 text-sm font-medium">
                <span>{strings.versions.name(version.version_number)}</span>
                {index === 0 && <Badge variant="secondary">{strings.versions.current}</Badge>}
              </p>
              <p className="text-muted-foreground text-xs">
                {formatBytes(version.size_bytes)} · {formatDate(version.created_at)}
              </p>
            </div>
            <Button
              variant="ghost"
              size="icon"
              aria-label={strings.versions.download(version.version_number)}
              onClick={() => download.mutate(version)}
            >
              <DownloadIcon />
            </Button>
          </li>
        ))}
      </ul>
    </>
  )
}
