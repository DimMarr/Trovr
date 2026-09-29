import { QueryClient } from '@tanstack/react-query'

export function createQueryClient(options: { retry?: boolean } = {}) {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: options.retry === false ? false : 1, refetchOnWindowFocus: false },
      mutations: { retry: false },
    },
  })
}
