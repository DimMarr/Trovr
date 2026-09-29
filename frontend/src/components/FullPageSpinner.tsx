import { Loader2Icon } from 'lucide-react'

import { strings } from '@/strings'

export function FullPageSpinner() {
  return (
    <div className="flex min-h-svh items-center justify-center" role="status">
      <Loader2Icon className="text-muted-foreground size-6 animate-spin" />
      <span className="sr-only">{strings.loading}</span>
    </div>
  )
}
