import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { describe, expect, test } from 'vitest'

import type { Node } from '@/api/types'
import { signedIn } from '@/test/auth'
import { renderApp } from '@/test/app'
import { file, folder } from '@/test/nodes'
import { server } from '@/test/server'

const trashedAt = '2026-09-03T10:00:00Z'

function liveTrash(initial: Node[]) {
  const state = { trash: initial }
  server.use(http.get('/api/v1/trash', () => HttpResponse.json(state.trash)))
  return state
}

function row(name: string) {
  return screen.getByText(name).closest('li')!
}

describe('trash', () => {
  test('lists trashed nodes with their trash date', async () => {
    signedIn()
    liveTrash([
      file('old.txt', { trashed_at: trashedAt }),
      folder('drafts', { trashed_at: trashedAt }),
    ])
    renderApp('/trash')

    expect(await screen.findByRole('heading', { name: 'Trash' })).toBeInTheDocument()
    expect(await screen.findByText('old.txt')).toBeInTheDocument()
    expect(within(row('old.txt')).getByText(/^Trashed /)).toBeInTheDocument()
  })

  test('restores a node', async () => {
    signedIn()
    const old = file('old.txt', { trashed_at: trashedAt })
    const state = liveTrash([old])
    let restored = false
    server.use(
      http.post(`/api/v1/nodes/${old.id}/restore`, () => {
        restored = true
        state.trash = []
        return HttpResponse.json({ ...old, trashed_at: null })
      }),
    )
    renderApp('/trash')

    await userEvent.click(await screen.findByRole('button', { name: 'Restore old.txt' }))

    expect(await screen.findByText('Restored “old.txt”.')).toBeInTheDocument()
    await waitFor(() => expect(screen.queryByText('old.txt')).not.toBeInTheDocument())
    expect(restored).toBe(true)
  })

  test.each([
    ['parent_trashed', 'Its folder is in the trash: restore the folder first.'],
    ['name_conflict', 'Something with this name already exists here.'],
  ])('explains a %s restore conflict', async (code, message) => {
    signedIn()
    const old = file('old.txt', { trashed_at: trashedAt })
    liveTrash([old])
    server.use(
      http.post(`/api/v1/nodes/${old.id}/restore`, () =>
        HttpResponse.json({ error: { code, message: code } }, { status: 409 }),
      ),
    )
    renderApp('/trash')

    await userEvent.click(await screen.findByRole('button', { name: 'Restore old.txt' }))

    expect(await screen.findByText(message)).toBeInTheDocument()
  })

  test('purges only after confirmation', async () => {
    signedIn()
    const old = file('old.txt', { trashed_at: trashedAt })
    const state = liveTrash([old])
    let purges = 0
    server.use(
      http.delete(`/api/v1/trash/${old.id}`, () => {
        purges += 1
        state.trash = []
        return new HttpResponse(null, { status: 204 })
      }),
    )
    renderApp('/trash')

    await userEvent.click(await screen.findByRole('button', { name: 'Delete old.txt forever' }))
    const confirm = await screen.findByRole('alertdialog')
    await userEvent.click(within(confirm).getByRole('button', { name: 'Cancel' }))
    expect(purges).toBe(0)

    await userEvent.click(screen.getByRole('button', { name: 'Delete old.txt forever' }))
    await userEvent.click(
      within(await screen.findByRole('alertdialog')).getByRole('button', {
        name: 'Delete forever',
      }),
    )

    await waitFor(() => expect(screen.queryByText('old.txt')).not.toBeInTheDocument())
    expect(purges).toBe(1)
  })

  test('shows an empty trash', async () => {
    signedIn()
    liveTrash([])
    renderApp('/trash')

    expect(await screen.findByText('The trash is empty.')).toBeInTheDocument()
  })
})
