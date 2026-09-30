import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'

import { shares } from '@/api/shares'
import type { GrantableRole } from '@/api/types'
import { errorMessage } from '@/lib/errors'

const sharesKey = (id: string) => ['shares', id] as const

const onError = (error: unknown) => {
  toast.error(errorMessage(error))
}

export function useShares(id: string) {
  return useQuery({ queryKey: sharesKey(id), queryFn: () => shares.list(id) })
}

/** Grants or changes a role; errors are left to the caller to show inline. */
export function useShare(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ email, role }: { email: string; role: GrantableRole }) =>
      shares.share(id, email, role),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: sharesKey(id) }),
  })
}

export function useUnshare(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (userId: string) => shares.unshare(id, userId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: sharesKey(id) }),
    onError,
  })
}

export function useCreateLink(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (expiresInSeconds: number | null) => shares.createLink(id, expiresInSeconds),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: sharesKey(id) }),
    onError,
  })
}

export function useDeleteLink(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (linkId: string) => shares.deleteLink(id, linkId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: sharesKey(id) }),
    onError,
  })
}
