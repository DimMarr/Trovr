import { ChevronRightIcon } from 'lucide-react'
import { Fragment } from 'react'
import { Link } from 'react-router'

export interface Crumb {
  label: string
  /** Absent for the current page. */
  to?: string
}

export function Breadcrumb({ crumbs }: { crumbs: Crumb[] }) {
  return (
    <nav aria-label="Breadcrumb" className="text-muted-foreground min-w-0 text-sm">
      <ol className="flex min-w-0 flex-wrap items-center gap-1">
        {crumbs.map((crumb, index) => (
          <Fragment key={`${index}-${crumb.label}`}>
            {index > 0 && <ChevronRightIcon className="size-3.5 shrink-0" aria-hidden />}
            <li className="min-w-0 truncate">
              {crumb.to ? (
                <Link to={crumb.to} className="hover:text-foreground hover:underline">
                  {crumb.label}
                </Link>
              ) : (
                <span aria-current="page" className="text-foreground font-medium">
                  {crumb.label}
                </span>
              )}
            </li>
          </Fragment>
        ))}
      </ol>
    </nav>
  )
}
