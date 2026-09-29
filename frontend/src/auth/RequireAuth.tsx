import { Navigate, Outlet, useLocation } from 'react-router'

import { FullPageSpinner } from '@/components/FullPageSpinner'

import { useAuth } from './context'

/** Renders the nested routes when signed in, otherwise sends the visitor to sign in. */
export function RequireAuth() {
  const { status } = useAuth()
  const location = useLocation()

  if (status === 'loading') return <FullPageSpinner />
  if (status === 'anonymous') {
    const next = encodeURIComponent(location.pathname + location.search)
    return <Navigate to={`/login?next=${next}`} replace />
  }
  return <Outlet />
}
