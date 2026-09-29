import { type FormEvent, useRef, useState } from 'react'

import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { strings } from '@/strings'

/** Asks for a name (new folder, rename); closes once `onSubmit` succeeds. */
export function NameDialog({
  open,
  onOpenChange,
  title,
  submitLabel,
  initialName = '',
  onSubmit,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  submitLabel: string
  initialName?: string
  onSubmit: (name: string) => Promise<unknown>
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        {/* Remounted on every opening so the field starts from `initialName`. */}
        {open && (
          <NameForm
            submitLabel={submitLabel}
            initialName={initialName}
            onSubmit={onSubmit}
            onDone={() => onOpenChange(false)}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

function NameForm({
  submitLabel,
  initialName,
  onSubmit,
  onDone,
}: {
  submitLabel: string
  initialName: string
  onSubmit: (name: string) => Promise<unknown>
  onDone: () => void
}) {
  const [name, setName] = useState(initialName)
  const [pending, setPending] = useState(false)
  // Guards against a second submit before the disabled state renders.
  const submitting = useRef(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    const trimmed = name.trim()
    if (!trimmed || submitting.current) return
    submitting.current = true
    setPending(true)
    try {
      await onSubmit(trimmed)
      onDone()
    } catch {
      // The mutation already reported the error; keep the dialog open to fix the name.
    } finally {
      submitting.current = false
      setPending(false)
    }
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <div className="grid gap-2">
        <Label htmlFor="node-name">{strings.name}</Label>
        <Input
          id="node-name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          autoFocus
          onFocus={(event) => {
            // Select the name without its extension, like file managers do.
            const dot = event.target.value.lastIndexOf('.')
            event.target.setSelectionRange(0, dot > 0 ? dot : event.target.value.length)
          }}
        />
      </div>
      <DialogFooter>
        <Button type="submit" disabled={pending || !name.trim()}>
          {submitLabel}
        </Button>
      </DialogFooter>
    </form>
  )
}
