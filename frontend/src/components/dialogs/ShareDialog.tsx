import { CopyIcon, LinkIcon, Trash2Icon, XIcon } from 'lucide-react'
import { type FormEvent, useState } from 'react'
import { toast } from 'sonner'

import { ApiError } from '@/api/client'
import type { GrantableRole, Link, Node, Share } from '@/api/types'
import { FormError } from '@/components/FormError'
import { NativeSelect } from '@/components/NativeSelect'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Separator } from '@/components/ui/separator'
import { useCreateLink, useDeleteLink, useShare, useShares, useUnshare } from '@/hooks/shares'
import { errorMessage } from '@/lib/errors'
import { formatDate } from '@/lib/format'
import { strings } from '@/strings'

const DAY = 24 * 60 * 60

/** Share a node with people (by email) and through public read-only links. */
export function ShareDialog({
  node,
  onOpenChange,
}: {
  node: Node | null
  onOpenChange: (open: boolean) => void
}) {
  return (
    <Dialog open={node !== null} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        {node && (
          <>
            <DialogHeader>
              <DialogTitle className="truncate">{strings.share.title(node.name)}</DialogTitle>
              <DialogDescription>{strings.share.description}</DialogDescription>
            </DialogHeader>
            <People node={node} />
            <Separator />
            <Links node={node} />
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}

function shareError(error: unknown): string | null {
  if (!error) return null
  // The only 422 the form can cause: the person already owns the node (or a folder above it).
  if (error instanceof ApiError && error.status === 422) return strings.share.alreadyOwner
  return errorMessage(error)
}

function People({ node }: { node: Node }) {
  const list = useShares(node.id)
  const share = useShare(node.id)
  const unshare = useUnshare(node.id)
  const [email, setEmail] = useState('')
  const [role, setRole] = useState<GrantableRole>('viewer')

  function submit(event: FormEvent) {
    event.preventDefault()
    share.mutate({ email: email.trim(), role }, { onSuccess: () => setEmail('') })
  }

  return (
    <section className="flex flex-col gap-3" aria-label={strings.share.people}>
      <form onSubmit={submit} className="flex flex-col gap-2">
        <div className="flex items-end gap-2">
          <div className="grid flex-1 gap-2">
            <Label htmlFor="share-email">{strings.email}</Label>
            <Input
              id="share-email"
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder="name@example.com"
              required
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="share-role">{strings.share.access}</Label>
            <NativeSelect
              id="share-role"
              value={role}
              onChange={(event) => setRole(event.target.value as GrantableRole)}
            >
              <option value="viewer">{strings.share.roles.viewer}</option>
              <option value="editor">{strings.share.roles.editor}</option>
            </NativeSelect>
          </div>
          <Button type="submit" disabled={share.isPending || !email.trim()}>
            {strings.share.submit}
          </Button>
        </div>
        <FormError message={shareError(share.error)} />
      </form>
      <ul className="flex flex-col gap-2">
        {(list.data?.users ?? []).map((entry) => (
          <Person
            key={entry.user.id}
            entry={entry}
            onRoleChange={(next) => share.mutate({ email: entry.user.email, role: next })}
            onRemove={() => unshare.mutate(entry.user.id)}
          />
        ))}
      </ul>
    </section>
  )
}

function Person({
  entry,
  onRoleChange,
  onRemove,
}: {
  entry: Share
  onRoleChange: (role: GrantableRole) => void
  onRemove: () => void
}) {
  const name = entry.user.display_name
  return (
    <li className="flex items-center gap-2">
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{name}</p>
        <p className="text-muted-foreground truncate text-xs">{entry.user.email}</p>
      </div>
      <NativeSelect
        aria-label={strings.share.accessFor(name)}
        value={entry.role}
        onChange={(event) => onRoleChange(event.target.value as GrantableRole)}
      >
        <option value="viewer">{strings.share.roles.viewer}</option>
        <option value="editor">{strings.share.roles.editor}</option>
      </NativeSelect>
      <Button
        variant="ghost"
        size="icon"
        aria-label={strings.share.remove(name)}
        onClick={onRemove}
      >
        <XIcon />
      </Button>
    </li>
  )
}

function Links({ node }: { node: Node }) {
  const list = useShares(node.id)
  const createLink = useCreateLink(node.id)
  const deleteLink = useDeleteLink(node.id)
  const [expiry, setExpiry] = useState('never')

  return (
    <section className="flex flex-col gap-3" aria-label={strings.share.links}>
      <div className="flex items-end gap-2">
        <div className="grid flex-1 gap-2">
          <Label htmlFor="link-expiry">{strings.share.linkExpires}</Label>
          <NativeSelect
            id="link-expiry"
            value={expiry}
            onChange={(event) => setExpiry(event.target.value)}
          >
            <option value="never">{strings.share.expiry.never}</option>
            <option value={String(DAY)}>{strings.share.expiry.day}</option>
            <option value={String(7 * DAY)}>{strings.share.expiry.week}</option>
            <option value={String(30 * DAY)}>{strings.share.expiry.month}</option>
          </NativeSelect>
        </div>
        <Button
          variant="outline"
          disabled={createLink.isPending}
          onClick={() => createLink.mutate(expiry === 'never' ? null : Number(expiry))}
        >
          <LinkIcon />
          {strings.share.createLink}
        </Button>
      </div>
      <ul className="flex flex-col gap-3">
        {(list.data?.links ?? []).map((link) => (
          <PublicLink key={link.id} link={link} onRevoke={() => deleteLink.mutate(link.id)} />
        ))}
      </ul>
    </section>
  )
}

function PublicLink({ link, onRevoke }: { link: Link; onRevoke: () => void }) {
  const url = `${window.location.origin}/s/${link.token}`

  async function copy() {
    try {
      await navigator.clipboard.writeText(url)
      toast.success(strings.share.copied)
    } catch {
      toast.error(strings.share.copyFailed)
    }
  }

  return (
    <li className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        <Input
          readOnly
          value={url}
          aria-label={strings.share.linkUrl}
          onFocus={(e) => e.target.select()}
        />
        <Button
          variant="outline"
          size="icon"
          aria-label={strings.share.copyLink}
          onClick={() => void copy()}
        >
          <CopyIcon />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          aria-label={strings.share.revokeLink}
          onClick={onRevoke}
        >
          <Trash2Icon />
        </Button>
      </div>
      <p className="text-muted-foreground text-xs">
        {link.expires_at
          ? strings.share.expiresOn(formatDate(link.expires_at))
          : strings.share.neverExpires}
      </p>
    </li>
  )
}
