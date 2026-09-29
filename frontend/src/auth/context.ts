import { createContext, useContext } from 'react'

import type { AuthConfig, User } from '@/api/types'

export interface AuthState {
  status: 'loading' | 'anonymous' | 'authenticated'
  user: User | null
  config: AuthConfig | null
  loginInternal(email: string, password: string): Promise<void>
  register(email: string, password: string, displayName: string): Promise<void>
  loginOidc(next: string): Promise<void>
  /** Finishes an OIDC sign-in; resolves to where the user wanted to go. */
  completeOidc(): Promise<string>
  logout(): Promise<void>
}

export const AuthContext = createContext<AuthState | null>(null)

export function useAuth(): AuthState {
  const state = useContext(AuthContext)
  if (!state) throw new Error('useAuth must be used inside <AuthProvider>')
  return state
}
