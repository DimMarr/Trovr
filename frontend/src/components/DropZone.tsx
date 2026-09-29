import { type DragEvent, type ReactNode, useState } from 'react'

import { cn } from '@/lib/utils'
import { strings } from '@/strings'

/** Accepts files dropped onto its area when `enabled`. */
export function DropZone({
  enabled,
  onFiles,
  children,
}: {
  enabled: boolean
  onFiles: (files: File[]) => void
  children: ReactNode
}) {
  const [dragging, setDragging] = useState(false)
  const carriesFiles = (event: DragEvent) => Array.from(event.dataTransfer.types).includes('Files')

  return (
    <div
      data-testid="drop-zone"
      className={cn(
        'relative min-h-64 rounded-lg',
        dragging && 'ring-primary ring-2 ring-offset-2',
      )}
      onDragOver={(event) => {
        if (!enabled || !carriesFiles(event)) return
        event.preventDefault()
        setDragging(true)
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(event) => {
        event.preventDefault()
        setDragging(false)
        if (enabled) onFiles(Array.from(event.dataTransfer.files))
      }}
    >
      {children}
      {dragging && (
        <div className="bg-background/80 pointer-events-none absolute inset-0 flex items-center justify-center rounded-lg">
          <p className="font-medium">{strings.uploads.dropHere}</p>
        </div>
      )}
    </div>
  )
}
