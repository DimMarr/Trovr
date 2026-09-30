import { render } from '@testing-library/react'
import type { ReactElement } from 'react'
import { MemoryRouter } from 'react-router'

import { Providers } from '@/components/Providers'
import { createQueryClient } from '@/lib/query'

/** Renders `ui` with the app's providers, at `route`, with retries off. */
export function renderWithProviders(ui: ReactElement, { route = '/' }: { route?: string } = {}) {
  const queryClient = createQueryClient({ retry: false })
  return {
    queryClient,
    ...render(
      <MemoryRouter initialEntries={[route]}>
        <Providers queryClient={queryClient}>{ui}</Providers>
      </MemoryRouter>,
    ),
  }
}
