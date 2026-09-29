import { XIcon } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { type UploadItem, useUploads } from '@/transfers/context'
import { strings } from '@/strings'

function StatusLine({ item }: { item: UploadItem }) {
  const { retry, replace, dismiss } = useUploads()
  switch (item.status) {
    case 'queued':
      return <p className="text-muted-foreground text-xs">{strings.uploads.waiting}</p>
    case 'uploading':
      return <Progress value={Math.round(item.progress * 100)} aria-label={item.file.name} />
    case 'confirming':
      return <p className="text-muted-foreground text-xs">{strings.uploads.finishing}</p>
    case 'done':
      return <p className="text-muted-foreground text-xs">{strings.uploads.done}</p>
    case 'error':
      return (
        <div className="flex items-center justify-between gap-2">
          <p className="text-destructive text-xs">{item.error}</p>
          {item.retryable && (
            <Button size="xs" variant="outline" onClick={() => retry(item.id)}>
              {strings.uploads.retry}
            </Button>
          )}
        </div>
      )
    case 'conflict':
      return (
        <div className="flex flex-col gap-1">
          <p className="text-xs">{strings.uploads.conflict}</p>
          <div className="flex gap-2">
            <Button size="xs" onClick={() => replace(item.id)}>
              {strings.uploads.replace}
            </Button>
            <Button size="xs" variant="outline" onClick={() => dismiss(item.id)}>
              {strings.uploads.skip}
            </Button>
          </div>
        </div>
      )
  }
}

/** Floating list of uploads in progress and their outcome. */
export function UploadPanel() {
  const { items, dismiss, clearFinished } = useUploads()
  if (items.length === 0) return null
  const active = items.some((item) => ['queued', 'uploading', 'confirming'].includes(item.status))

  return (
    <section
      aria-label={strings.uploads.title}
      className="bg-background fixed right-4 bottom-4 z-40 flex max-h-96 w-80 flex-col rounded-lg border shadow-lg"
    >
      <header className="flex items-center justify-between border-b px-3 py-2">
        <h2 className="text-sm font-medium">{strings.uploads.title}</h2>
        {!active && (
          <Button size="icon-xs" variant="ghost" onClick={clearFinished} aria-label={strings.close}>
            <XIcon />
          </Button>
        )}
      </header>
      <ul className="flex flex-col gap-3 overflow-y-auto p-3">
        {items.map((item) => (
          <li key={item.id} className="flex flex-col gap-1">
            <div className="flex items-center justify-between gap-2">
              <p className="truncate text-sm">{item.file.name}</p>
              {(item.status === 'error' || item.status === 'done') && (
                <Button
                  size="icon-xs"
                  variant="ghost"
                  onClick={() => dismiss(item.id)}
                  aria-label={strings.uploads.dismiss(item.file.name)}
                >
                  <XIcon />
                </Button>
              )}
            </div>
            <StatusLine item={item} />
          </li>
        ))}
      </ul>
    </section>
  )
}
