/** How the user signed in; decides how to sign out. */
export type SignInMethod = 'internal' | 'oidc'

export interface Session {
  token: string
  method: SignInMethod
}

const KEY = 'trovr.session'

/** The session in memory; `sessionStorage` only mirrors it across reloads. */
let current: Session | null = null

export function currentSession(): Session | null {
  return current
}

/** The stored session, if any and not expired. Survives reloads, not closing the tab. */
export function loadSession(): Session | null {
  try {
    const session = JSON.parse(sessionStorage.getItem(KEY) ?? 'null') as Session | null
    current = session?.token && !isExpired(session.token) ? session : null
    if (!current) sessionStorage.removeItem(KEY)
  } catch {
    current = null
  }
  return current
}

export function saveSession(session: Session | null) {
  current = session
  try {
    if (session) sessionStorage.setItem(KEY, JSON.stringify(session))
    else sessionStorage.removeItem(KEY)
  } catch {
    // Storage can be unavailable (private mode); the session then lasts until reload.
  }
}

/** The token's `exp` claim in milliseconds, or null when it has none. */
export function tokenExpiry(token: string): number | null {
  try {
    const payload = token.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')
    const { exp } = JSON.parse(atob(payload)) as { exp?: number }
    return typeof exp === 'number' ? exp * 1000 : null
  } catch {
    return null
  }
}

function isExpired(token: string) {
  const expiry = tokenExpiry(token)
  return expiry !== null && expiry <= Date.now()
}
