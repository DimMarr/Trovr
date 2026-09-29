import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'

import { files } from '@/api/files'
import type { Node } from '@/api/types'
import { errorMessage } from '@/lib/errors'
import { uploadContent } from '@/transfers/upload'

import { keys } from './nodes'

/** A file's versions, newest first. */
export function useVersions(id: string | null) {
  return useQuery({
    queryKey: ['versions', id],
    queryFn: () => files.versions(id!),
    enabled: id !== null,
    select: (versions) => [...versions].sort((a, b) => b.version_number - a.version_number),
  })
}

/** Uploads `file` straight to storage, then records it as the node's new current version. */
export function useAddVersion(node: Node) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async (file: File) => files.createVersion(node.id, await uploadContent(file)),
    onSuccess: () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: ['versions', node.id] }),
        queryClient.invalidateQueries({ queryKey: keys.children(node.parent_id) }),
      ]),
    onError: (error) => {
      toast.error(errorMessage(error))
    },
  })
}
