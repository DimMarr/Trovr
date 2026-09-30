import { request } from './client'
import type { Node } from './types'

export const trash = {
  list: () => request<Node[]>('GET', '/trash'),
  restore: (id: string) => request<Node>('POST', `/nodes/${id}/restore`, { body: {} }),
  purge: (id: string) => request<void>('DELETE', `/trash/${id}`),
}
