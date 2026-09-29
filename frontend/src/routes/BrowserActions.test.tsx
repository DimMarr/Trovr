import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse, delay } from 'msw'
import { describe, expect, test } from 'vitest'

import type { Node } from '@/api/types'
import { signedIn } from '@/test/auth'
import { renderApp } from '@/test/app'
import { file, folder, mockFolder } from '@/test/nodes'
import { server } from '@/test/server'

async function openActions(name: string) {
  await userEvent.click(await screen.findByRole('button', { name: `Actions for ${name}` }))
}

/** Serves `/nodes` from a mutable list so refetches see changes. */
function liveRoot(initial: Node[]) {
  let children = initial
  server.use(http.get('/api/v1/nodes', () => HttpResponse.json(children)))
  return (next: Node[]) => {
    children = next
  }
}

describe('folder actions', () => {
  test('creates a folder in the current folder', async () => {
    signedIn()
    const docs = folder('docs')
    const created = folder('drafts', { parent_id: docs.id })
    let children: Node[] = []
    let body: unknown = null
    mockFolder(docs)
    server.use(
      http.get(`/api/v1/nodes/${docs.id}/children`, () => HttpResponse.json(children)),
      http.post('/api/v1/folders', async ({ request }) => {
        body = await request.json()
        children = [created]
        return HttpResponse.json(created, { status: 201 })
      }),
    )
    renderApp(`/folders/${docs.id}`)

    await userEvent.click(await screen.findByRole('button', { name: 'New folder' }))
    const dialog = await screen.findByRole('dialog')
    await userEvent.type(within(dialog).getByLabelText('Name'), 'drafts')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Create' }))

    expect(await screen.findByRole('button', { name: 'drafts' })).toBeInTheDocument()
    expect(body).toEqual({ parent_id: docs.id, name: 'drafts' })
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  })

  test('renames a node', async () => {
    signedIn()
    const report = file('report.txt')
    const setRoot = liveRoot([report])
    let body: unknown = null
    server.use(
      http.post(`/api/v1/nodes/${report.id}/rename`, async ({ request }) => {
        body = await request.json()
        const renamed = { ...report, name: 'final.txt' }
        setRoot([renamed])
        return HttpResponse.json(renamed)
      }),
    )
    renderApp('/')

    await openActions('report.txt')
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Rename' }))
    const input = within(await screen.findByRole('dialog')).getByLabelText('Name')
    expect(input).toHaveValue('report.txt')
    await userEvent.clear(input)
    await userEvent.type(input, 'final.txt')
    await userEvent.click(screen.getByRole('button', { name: 'Rename' }))

    expect(await screen.findByRole('button', { name: 'final.txt' })).toBeInTheDocument()
    expect(body).toEqual({ name: 'final.txt' })
  })

  test('moves a node into another folder', async () => {
    signedIn()
    const a = folder('a')
    const b = folder('b')
    const setRoot = liveRoot([a, b])
    mockFolder(b)
    let body: unknown = null
    server.use(
      http.post(`/api/v1/nodes/${a.id}/move`, async ({ request }) => {
        body = await request.json()
        setRoot([b])
        return HttpResponse.json({ ...a, parent_id: b.id })
      }),
    )
    renderApp('/')

    await openActions('a')
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Move' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).queryByRole('button', { name: 'a' })).not.toBeInTheDocument()
    await userEvent.click(await within(dialog).findByRole('button', { name: 'b' }))
    await userEvent.click(within(dialog).getByRole('button', { name: 'Move here' }))

    await waitFor(() => expect(screen.queryByRole('button', { name: 'a' })).not.toBeInTheDocument())
    expect(body).toEqual({ parent_id: b.id })
  })

  test('moves a node to the trash', async () => {
    signedIn()
    const old = file('old.txt')
    const setRoot = liveRoot([old])
    server.use(
      http.delete(`/api/v1/nodes/${old.id}`, () => {
        setRoot([])
        return HttpResponse.json({ ...old, trashed_at: '2026-09-03T10:00:00Z' })
      }),
    )
    renderApp('/')

    await openActions('old.txt')
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Move to trash' }))

    expect(await screen.findByText('Moved “old.txt” to the trash.')).toBeInTheDocument()
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: 'old.txt' })).not.toBeInTheDocument(),
    )
  })

  test('shows name conflicts as a toast and keeps the dialog open', async () => {
    signedIn()
    liveRoot([])
    server.use(
      http.post('/api/v1/folders', () =>
        HttpResponse.json(
          { error: { code: 'name_conflict', message: 'a node with this name already exists' } },
          { status: 409 },
        ),
      ),
    )
    renderApp('/')

    await userEvent.click(await screen.findByRole('button', { name: 'New folder' }))
    await userEvent.type(within(await screen.findByRole('dialog')).getByLabelText('Name'), 'docs')
    await userEvent.click(screen.getByRole('button', { name: 'Create' }))

    expect(
      await screen.findByText('Something with this name already exists here.'),
    ).toBeInTheDocument()
    expect(screen.getByRole('dialog')).toBeInTheDocument()
  })

  test('viewers see no write actions', async () => {
    signedIn()
    const shared = folder('shared')
    mockFolder(shared, { role: 'viewer', children: [file('x.txt', { parent_id: shared.id })] })
    renderApp(`/folders/${shared.id}`)

    await screen.findByRole('button', { name: 'x.txt' })
    expect(screen.queryByRole('button', { name: 'New folder' })).not.toBeInTheDocument()
    const menu = screen.queryByRole('button', { name: 'Actions for x.txt' })
    if (menu) {
      await userEvent.click(menu)
      for (const item of ['Rename', 'Move', 'Move to trash']) {
        expect(screen.queryByRole('menuitem', { name: item })).not.toBeInTheDocument()
      }
    }
  })

  test('dialogs submit once while pending', async () => {
    signedIn()
    liveRoot([])
    let posts = 0
    server.use(
      http.post('/api/v1/folders', async () => {
        posts += 1
        await delay(100)
        return HttpResponse.json(folder('docs'), { status: 201 })
      }),
    )
    renderApp('/')

    await userEvent.click(await screen.findByRole('button', { name: 'New folder' }))
    await userEvent.type(within(await screen.findByRole('dialog')).getByLabelText('Name'), 'docs')
    await userEvent.dblClick(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(posts).toBe(1)
  })
})
