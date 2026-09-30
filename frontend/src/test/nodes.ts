import { http, HttpResponse } from 'msw'

import type { Node, NodeWithRole, Role } from '@/api/types'

import { server } from './server'

let counter = 0

export function folder(name: string, overrides: Partial<Node> = {}): Node {
  return node({ name, type: 'folder', size_bytes: 0, mime_type: null, ...overrides })
}

export function file(name: string, overrides: Partial<Node> = {}): Node {
  return node({ name, type: 'file', size_bytes: 1536, mime_type: 'text/plain', ...overrides })
}

function node(fields: Partial<Node> & Pick<Node, 'name' | 'type'>): Node {
  counter += 1
  return {
    id: `${fields.name}-${counter}`,
    parent_id: null,
    size_bytes: 0,
    mime_type: null,
    created_at: '2026-09-01T10:00:00Z',
    updated_at: '2026-09-02T10:00:00Z',
    trashed_at: null,
    ...fields,
  }
}

export function mockRoot(children: Node[]) {
  server.use(http.get('/api/v1/nodes', () => HttpResponse.json(children)))
}

/** Mocks a folder page: the node with the caller's role, its children and its path. */
export function mockFolder(
  target: Node,
  {
    children = [],
    path = [target],
    role = 'owner',
  }: { children?: Node[]; path?: Node[]; role?: Role } = {},
) {
  const withRole: NodeWithRole = { ...target, role }
  server.use(
    http.get(`/api/v1/nodes/${target.id}`, () => HttpResponse.json(withRole)),
    http.get(`/api/v1/nodes/${target.id}/children`, () => HttpResponse.json(children)),
    http.get(`/api/v1/nodes/${target.id}/path`, () => HttpResponse.json(path)),
  )
}

export function mockMissingFolder(id: string, status = 404) {
  const body = { error: { code: status === 403 ? 'forbidden' : 'not_found', message: 'nope' } }
  server.use(
    http.get(`/api/v1/nodes/${id}`, () => HttpResponse.json(body, { status })),
    http.get(`/api/v1/nodes/${id}/children`, () => HttpResponse.json(body, { status })),
    http.get(`/api/v1/nodes/${id}/path`, () => HttpResponse.json(body, { status })),
  )
}
