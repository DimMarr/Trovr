import { request } from './client'
import type { Node, Presigned } from './types'

/** Read-only access through a public link: no account, the token is the credential. */
export const publicLinks = {
  root: (token: string) => request<Node>('GET', `/public/${token}`, { auth: false }),
  children: (token: string, id: string) =>
    request<Node[]>('GET', `/public/${token}/nodes/${id}/children`, { auth: false }),
  download: (token: string, id: string) =>
    request<Presigned>('GET', `/public/${token}/nodes/${id}/download`, { auth: false }),
}
