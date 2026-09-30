import type { ComponentProps } from 'react'

import { cn } from '@/lib/utils'

/** A native `<select>` styled like the other form fields. */
export function NativeSelect({ className, ...props }: ComponentProps<'select'>) {
  return (
    <select
      className={cn(
        'border-input bg-background focus-visible:ring-ring/50 h-8 rounded-md border px-2 text-sm outline-none focus-visible:ring-3 disabled:opacity-50',
        className,
      )}
      {...props}
    />
  )
}
