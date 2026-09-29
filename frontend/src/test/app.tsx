import { screen } from '@testing-library/react'

import { AppRoutes } from '@/App'
import { AuthProvider } from '@/auth/AuthProvider'
import type { OidcClient } from '@/auth/oidc'

import { LocationProbe, fakeOidcClient } from './auth'
import { renderWithProviders } from './render'

/** Renders the whole app (routes + auth) at `route`. */
export function renderApp(route: string, oidc: OidcClient = fakeOidcClient()) {
  return renderWithProviders(
    <AuthProvider createOidcClient={() => oidc}>
      <AppRoutes />
      <LocationProbe />
    </AuthProvider>,
    { route },
  )
}

export const currentLocation = () => screen.getByTestId('location').textContent
