import { FolderIcon, Trash2Icon, UsersIcon } from 'lucide-react'
import { NavLink, Outlet } from 'react-router'

import { UserMenu } from '@/components/UserMenu'
import { cn } from '@/lib/utils'
import { strings } from '@/strings'

const links = [
  { to: '/', label: strings.myFiles, icon: FolderIcon, end: false },
  { to: '/shared', label: strings.sharedWithMe, icon: UsersIcon, end: true },
  { to: '/trash', label: strings.trash, icon: Trash2Icon, end: true },
]

/** The signed-in layout: navigation sidebar and the current page. */
export function AppLayout() {
  return (
    <div className="flex min-h-svh flex-col md:flex-row">
      <aside className="bg-muted/40 flex flex-col gap-4 border-b p-4 md:w-60 md:border-r md:border-b-0">
        <p className="px-2 text-lg font-semibold">{strings.appName}</p>
        <nav aria-label="Main" className="flex gap-1 md:flex-1 md:flex-col">
          {links.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                cn(
                  'hover:bg-accent flex items-center gap-2 rounded-md px-2 py-1.5 text-sm',
                  isActive && 'bg-accent font-medium',
                )
              }
            >
              <Icon className="size-4" aria-hidden />
              {label}
            </NavLink>
          ))}
        </nav>
        <UserMenu />
      </aside>
      <main className="min-w-0 flex-1 p-4 md:p-6">
        <Outlet />
      </main>
    </div>
  )
}
