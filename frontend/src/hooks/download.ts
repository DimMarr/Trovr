import { useMutation } from '@tanstack/react-query'
import { toast } from 'sonner'

import type { Presigned } from '@/api/types'
import { errorMessage } from '@/lib/errors'
import { startDownload } from '@/transfers/download'

/** Asks for a presigned GET with `presign`, then lets the browser download it. */
export function useDownload<T>(presign: (target: T) => Promise<Presigned>) {
  return useMutation({
    mutationFn: async (target: T) => startDownload(await presign(target)),
    onError: (error) => {
      toast.error(errorMessage(error))
    },
  })
}
