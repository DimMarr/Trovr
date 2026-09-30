import { useQuery } from '@tanstack/react-query'

import { publicLinks } from '@/api/public'

export function usePublicRoot(token: string) {
  return useQuery({ queryKey: ['public', token], queryFn: () => publicLinks.root(token) })
}

export function usePublicChildren(token: string, id: string | null) {
  return useQuery({
    queryKey: ['public', token, 'children', id],
    queryFn: () => publicLinks.children(token, id!),
    enabled: id !== null,
  })
}
