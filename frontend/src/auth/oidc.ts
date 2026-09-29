import { UserManager, WebStorageStateStore } from 'oidc-client-ts'

/** The OIDC operations the app needs, so tests can swap the library out. */
export interface OidcClient {
  /** Redirects to the identity provider; `next` is where to land afterwards. */
  signIn(next: string): Promise<void>
  /** Finishes the redirect flow on `/auth/callback`. */
  completeSignIn(): Promise<{ accessToken: string; next: string }>
  /** Signs out at the identity provider when it supports it. */
  signOut(): Promise<void>
  /** Called with each silently renewed access token; returns an unsubscribe function. */
  onTokenRenewed(callback: (accessToken: string) => void): () => void
}

export interface OidcSettings {
  issuer: string
  client_id: string
}

export function createOidcClient({ issuer, client_id }: OidcSettings): OidcClient {
  const origin = window.location.origin
  const manager = new UserManager({
    authority: issuer,
    client_id,
    redirect_uri: `${origin}/auth/callback`,
    post_logout_redirect_uri: `${origin}/login`,
    scope: 'openid profile email',
    automaticSilentRenew: true,
    userStore: new WebStorageStateStore({ store: window.sessionStorage }),
  })

  return {
    signIn: (next) => manager.signinRedirect({ state: { next } }),
    async completeSignIn() {
      const user = await manager.signinRedirectCallback()
      const state = user.state as { next?: string } | undefined
      return { accessToken: user.access_token, next: state?.next ?? '/' }
    },
    async signOut() {
      try {
        await manager.signoutRedirect()
      } catch {
        // The provider has no end-session endpoint: forget the user locally.
        await manager.removeUser()
      }
    },
    onTokenRenewed: (callback) =>
      manager.events.addUserLoaded((user) => callback(user.access_token)),
  }
}
