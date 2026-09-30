import type { ReactNode } from 'react'

export function PageHeader({
  title,
  breadcrumb,
  actions,
}: {
  title: string
  breadcrumb?: ReactNode
  actions?: ReactNode
}) {
  return (
    <header className="mb-4 flex flex-col gap-2">
      {breadcrumb}
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h1 className="min-w-0 truncate text-2xl font-semibold">{title}</h1>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </div>
    </header>
  )
}
