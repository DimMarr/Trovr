import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'

import { trash } from '@/api/trash'
import type { Node } from '@/api/types'
import { errorMessage } from '@/lib/errors'
import { strings } from '@/strings'

import { keys } from './nodes'

const onError = (error: unknown) => {
  toast.error(errorMessage(error))
}

export function useTrashList() {
  return useQuery({ queryKey: ['trash'], queryFn: trash.list })
}

export function useRestore() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (node: Node) => trash.restore(node.id),
    onSuccess: (_, node) => {
      toast.success(strings.trashView.restored(node.name))
      return Promise.all([
        queryClient.invalidateQueries({ queryKey: ['trash'] }),
        queryClient.invalidateQueries({ queryKey: keys.children(node.parent_id) }),
      ])
    },
    onError,
  })
}

export function usePurge() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (node: Node) => trash.purge(node.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['trash'] }),
    onError,
  })
}
