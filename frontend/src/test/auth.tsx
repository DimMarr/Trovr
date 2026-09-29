import { http, HttpResponse } from 'msw'
import { useLocation } from 'react-router'
import { vi } from 'vitest'

import type { AuthConfig, User } from '@/api/types'
import type { OidcClient } from '@/auth/oidc'

import { server } from './server'

export const alice: User = {
  id: 'u-alice',
  email: 'alice@example.com',
  display_name: 'Alice',
  issuer: 'internal',
}

/** An unsigned JWT: the client only reads its `exp` claim. */
export function fakeJwt(expiresInSeconds = 3600): string {
  const encode = (value: object) => btoa(JSON.stringify(value)).replace(/=+$/, '')
  const exp = Math.floor(Date.now() / 1000) + expiresInSeconds
  return `${encode({ alg: 'RS256' })}.${encode({ sub: alice.id, exp })}.signature`
}

export function mockAuthConfig(overrides: Partial<AuthConfig> = {}) {
  const config: AuthConfig = {
    internal: { enabled: true, registration: false },
    oidc: null,
    max_upload_bytes: 1024 * 1024,
    ...overrides,
  }
  server.use(http.get('/api/v1/auth/config', () => HttpResponse.json(config)))
  return config
}

export function mockMe(user: User = alice) {
  server.use(http.get('/api/v1/me', () => HttpResponse.json(user)))
}

/** Starts the test as if the user had signed in before a reload. */
export function storeSession(token = fakeJwt(), method: 'internal' | 'oidc' = 'internal') {
  sessionStorage.setItem('trovr.session', JSON.stringify({ token, method }))
}

/** Signed in with internal auth, config and `/me` mocked. */
export function signedIn(user: User = alice) {
  mockAuthConfig()
  mockMe(user)
  storeSession()
}

export function fakeOidcClient(overrides: Partial<OidcClient> = {}): OidcClient {
  return {
    signIn: vi.fn(async () => {}),
    completeSignIn: vi.fn(async () => ({ accessToken: fakeJwt(), next: '/' })),
    signOut: vi.fn(async () => {}),
    onTokenRenewed: vi.fn(() => () => {}),
    ...overrides,
  }
}

/** Renders the current location so tests can assert on redirects. */
export function LocationProbe() {
  const location = useLocation()
  return <output data-testid="location">{location.pathname + location.search}</output>
}
