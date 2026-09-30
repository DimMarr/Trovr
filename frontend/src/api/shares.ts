import { request } from './client'
import type { GrantableRole, Link, Node, Share, Shares } from './types'

export const shares = {
  list: (id: string) => request<Shares>('GET', `/nodes/${id}/shares`),
  share: (id: string, email: string, role: GrantableRole) =>
    request<Share>('POST', `/nodes/${id}/shares`, { body: { email, role } }),
  unshare: (id: string, userId: string) => request<void>('DELETE', `/nodes/${id}/shares/${userId}`),
  sharedWithMe: () => request<Node[]>('GET', '/shared'),
  createLink: (id: string, expiresInSeconds: number | null) =>
    request<Link>('POST', `/nodes/${id}/links`, {
      body: expiresInSeconds === null ? {} : { expires_in_seconds: expiresInSeconds },
    }),
  deleteLink: (id: string, linkId: string) =>
    request<void>('DELETE', `/nodes/${id}/links/${linkId}`),
}
