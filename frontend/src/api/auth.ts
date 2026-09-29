import { request } from './client'
import type { AuthConfig, User } from './types'

export const auth = {
  getConfig: () => request<AuthConfig>('GET', '/auth/config', { auth: false }),
  login: (email: string, password: string) =>
    request<{ access_token: string }>('POST', '/auth/login', {
      auth: false,
      body: { email, password },
    }),
  register: (email: string, password: string, display_name: string) =>
    request<User>('POST', '/auth/register', {
      auth: false,
      body: { email, password, display_name },
    }),
  me: () => request<User>('GET', '/me'),
}
