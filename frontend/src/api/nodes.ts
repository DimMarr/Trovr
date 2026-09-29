import { request } from './client'
import type { Node, NodeWithRole } from './types'

export const nodes = {
  listRoot: () => request<Node[]>('GET', '/nodes'),
  get: (id: string) => request<NodeWithRole>('GET', `/nodes/${id}`),
  children: (id: string) => request<Node[]>('GET', `/nodes/${id}/children`),
  path: (id: string) => request<Node[]>('GET', `/nodes/${id}/path`),
  createFolder: (parentId: string | null, name: string) =>
    request<Node>('POST', '/folders', { body: { parent_id: parentId, name } }),
  rename: (id: string, name: string) =>
    request<Node>('POST', `/nodes/${id}/rename`, { body: { name } }),
  move: (id: string, parentId: string | null) =>
    request<Node>('POST', `/nodes/${id}/move`, { body: { parent_id: parentId } }),
  trash: (id: string) => request<Node>('DELETE', `/nodes/${id}`),
}
