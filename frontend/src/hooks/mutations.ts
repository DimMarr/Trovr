import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'

import { nodes } from '@/api/nodes'
import type { Node } from '@/api/types'
import { errorMessage } from '@/lib/errors'
import { strings } from '@/strings'

import { keys } from './nodes'

const onError = (error: unknown) => {
  toast.error(errorMessage(error))
}

export function useCreateFolder(parentId: string | null) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (name: string) => nodes.createFolder(parentId, name),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: keys.children(parentId) }),
    onError,
  })
}

export function useRename() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ node, name }: { node: Node; name: string }) => nodes.rename(node.id, name),
    onSuccess: (_, { node }) =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: keys.children(node.parent_id) }),
        queryClient.invalidateQueries({ queryKey: keys.node(node.id) }),
        // Every breadcrumb below the node shows its name.
        queryClient.invalidateQueries({ queryKey: ['path'] }),
      ]),
    onError,
  })
}

export function useMove() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ node, parentId }: { node: Node; parentId: string | null }) =>
      nodes.move(node.id, parentId),
    onSuccess: (_, { node, parentId }) =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: keys.children(node.parent_id) }),
        queryClient.invalidateQueries({ queryKey: keys.children(parentId) }),
        queryClient.invalidateQueries({ queryKey: keys.node(node.id) }),
        queryClient.invalidateQueries({ queryKey: ['path'] }),
      ]),
    onError,
  })
}

export function useTrash() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (node: Node) => nodes.trash(node.id),
    onSuccess: (_, node) => {
      toast.success(strings.movedToTrash(node.name))
      return Promise.all([
        queryClient.invalidateQueries({ queryKey: keys.children(node.parent_id) }),
        queryClient.invalidateQueries({ queryKey: ['trash'] }),
      ])
    },
    onError,
  })
}
