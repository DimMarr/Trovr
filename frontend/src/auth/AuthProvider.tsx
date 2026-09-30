import { useQuery, useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useCallback, useEffect, useMemo, useState } from 'react'

import { auth } from '@/api/auth'
import { configureApi } from '@/api/client'

import { AuthContext, type AuthState } from './context'
import {
  type OidcClient,
  type OidcSettings,
  createOidcClient as createDefaultOidcClient,
} from './oidc'
import { type Session, currentSession, loadSession, saveSession, tokenExpiry } from './session'

export function AuthProvider({
  children,
  createOidcClient = createDefaultOidcClient,
}: {
  children: ReactNode
  createOidcClient?: (settings: OidcSettings) => OidcClient
}) {
  const queryClient = useQueryClient()
  const [session, setSessionState] = useState<Session | null>(loadSession)

  const setSession = useCallback(
    (next: Session | null) => {
      saveSession(next)
      setSessionState(next)
      if (!next) queryClient.removeQueries({ queryKey: ['me'] })
    },
    [queryClient],
  )

  // Configured on first render so that children's first requests already carry the token.
  useState(() =>
    configureApi({
      getToken: () => currentSession()?.token ?? null,
      onUnauthorized: () => setSession(null),
    }),
  )

  const config = useQuery({
    queryKey: ['auth-config'],
    queryFn: auth.getConfig,
    staleTime: Infinity,
  })
  const me = useQuery({
    queryKey: ['me'],
    queryFn: auth.me,
    enabled: session !== null,
    staleTime: Infinity,
  })

  const oidcSettings = config.data?.oidc
  const oidc = useMemo(
    () => (oidcSettings ? createOidcClient(oidcSettings) : null),
    [oidcSettings, createOidcClient],
  )

  // Silent renewals replace the access token of an OIDC session.
  useEffect(() => {
    if (!oidc) return
    return oidc.onTokenRenewed((token) => {
      if (currentSession()?.method === 'oidc') setSession({ token, method: 'oidc' })
    })
  }, [oidc, setSession])

  // Drop the session when its token expires.
  useEffect(() => {
    const expiry = session ? tokenExpiry(session.token) : null
    if (expiry === null) return
    const timer = window.setTimeout(() => setSession(null), Math.max(0, expiry - Date.now()))
    return () => window.clearTimeout(timer)
  }, [session, setSession])

  const value = useMemo<AuthState>(() => {
    const loading = config.isPending || (session !== null && me.isPending)
    return {
      status: loading ? 'loading' : session && me.data ? 'authenticated' : 'anonymous',
      user: session ? (me.data ?? null) : null,
      config: config.data ?? null,
      async loginInternal(email, password) {
        const { access_token } = await auth.login(email, password)
        setSession({ token: access_token, method: 'internal' })
      },
      async register(email, password, displayName) {
        await auth.register(email, password, displayName)
        const { access_token } = await auth.login(email, password)
        setSession({ token: access_token, method: 'internal' })
      },
      async loginOidc(next) {
        if (!oidc) throw new Error('OIDC is not enabled')
        await oidc.signIn(next)
      },
      async completeOidc() {
        if (!oidc) throw new Error('OIDC is not enabled')
        const { accessToken, next } = await oidc.completeSignIn()
        setSession({ token: accessToken, method: 'oidc' })
        return next
      },
      async logout() {
        const method = currentSession()?.method
        setSession(null)
        queryClient.clear()
        if (method === 'oidc' && oidc) await oidc.signOut()
      },
    }
  }, [config.isPending, config.data, session, me.isPending, me.data, oidc, setSession, queryClient])

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
