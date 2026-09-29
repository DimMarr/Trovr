import { useQuery } from '@tanstack/react-query'

import { nodes } from '@/api/nodes'

/** Query keys shared by queries and the mutations that invalidate them. */
export const keys = {
  node: (id: string) => ['node', id] as const,
  children: (id: string | null) => ['children', id ?? 'root'] as const,
  path: (id: string) => ['path', id] as const,
}

export function useNode(id: string | null) {
  return useQuery({
    queryKey: keys.node(id ?? ''),
    queryFn: () => nodes.get(id!),
    enabled: id !== null,
  })
}

/** The children of a folder, or the caller's root when `id` is null. */
export function useChildren(id: string | null) {
  return useQuery({
    queryKey: keys.children(id),
    queryFn: () => (id === null ? nodes.listRoot() : nodes.children(id)),
  })
}

export function usePath(id: string | null) {
  return useQuery({
    queryKey: keys.path(id ?? ''),
    queryFn: () => nodes.path(id!),
    enabled: id !== null,
  })
}
