import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { describe, expect, test } from 'vitest'

import type { Link, Node, Share, Shares } from '@/api/types'
import { signedIn } from '@/test/auth'
import { renderApp } from '@/test/app'
import { file, folder, mockFolder } from '@/test/nodes'
import { server } from '@/test/server'

const bob = { id: 'u-bob', email: 'bob@example.com', display_name: 'Bob' }

function share(role: Share['role']): Share {
  return { user: bob, role, created_at: '2026-09-01T10:00:00Z' }
}

function link(overrides: Partial<Link> = {}): Link {
  return {
    id: 'l1',
    token: 'tok123',
    role: 'viewer',
    expires_at: null,
    created_at: '2026-09-01T10:00:00Z',
    ...overrides,
  }
}

/** Root with one folder, and its shares served from a mutable state. */
function setup(initial: Shares = { users: [], links: [] }) {
  signedIn()
  const docs = folder('docs')
  const state = { shares: initial, node: docs as Node }
  server.use(
    http.get('/api/v1/nodes', () => HttpResponse.json([docs])),
    http.get(`/api/v1/nodes/${docs.id}/shares`, () => HttpResponse.json(state.shares)),
  )
  return state
}

async function openShare(name = 'docs') {
  await userEvent.click(await screen.findByRole('button', { name: `Actions for ${name}` }))
  await userEvent.click(await screen.findByRole('menuitem', { name: 'Share' }))
  return screen.findByRole('dialog')
}

function errorBody(status: number, code: string) {
  return HttpResponse.json(
    { error: { code, message: 'this user already owns the node' } },
    { status },
  )
}

describe('sharing with people', () => {
  test('shares with a user by email', async () => {
    const state = setup()
    let body: unknown = null
    server.use(
      http.post(`/api/v1/nodes/${state.node.id}/shares`, async ({ request }) => {
        body = await request.json()
        state.shares = { users: [share('editor')], links: [] }
        return HttpResponse.json(share('editor'), { status: 201 })
      }),
    )
    renderApp('/')

    const dialog = await openShare()
    await userEvent.type(within(dialog).getByLabelText('Email'), 'bob@example.com')
    await userEvent.selectOptions(within(dialog).getByLabelText('Access'), 'editor')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Share' }))

    expect(await within(dialog).findByText('Bob')).toBeInTheDocument()
    expect(body).toEqual({ email: 'bob@example.com', role: 'editor' })
    expect(within(dialog).getByLabelText('Email')).toHaveValue('')
  })

  test.each([
    [404, 'user_not_found', 'No account uses this email.'],
    [409, 'ambiguous_user', 'Several accounts use this email.'],
    [422, 'invalid_request', 'This person already owns it.'],
  ])('explains a %i %s', async (status, code, message) => {
    const state = setup()
    server.use(http.post(`/api/v1/nodes/${state.node.id}/shares`, () => errorBody(status, code)))
    renderApp('/')

    const dialog = await openShare()
    await userEvent.type(within(dialog).getByLabelText('Email'), 'someone@example.com')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Share' }))

    expect(await within(dialog).findByRole('alert')).toHaveTextContent(message)
  })

  test('changes a role and removes a person', async () => {
    const state = setup({ users: [share('viewer')], links: [] })
    const posted: unknown[] = []
    let removed = false
    server.use(
      http.post(`/api/v1/nodes/${state.node.id}/shares`, async ({ request }) => {
        posted.push(await request.json())
        state.shares = { users: [share('editor')], links: [] }
        return HttpResponse.json(share('editor'), { status: 201 })
      }),
      http.delete(`/api/v1/nodes/${state.node.id}/shares/${bob.id}`, () => {
        removed = true
        state.shares = { users: [], links: [] }
        return new HttpResponse(null, { status: 204 })
      }),
    )
    renderApp('/')

    const dialog = await openShare()
    await userEvent.selectOptions(await within(dialog).findByLabelText('Access for Bob'), 'editor')
    await waitFor(() => expect(posted).toEqual([{ email: bob.email, role: 'editor' }]))

    await userEvent.click(within(dialog).getByRole('button', { name: 'Remove Bob' }))
    await waitFor(() => expect(within(dialog).queryByText('Bob')).not.toBeInTheDocument())
    expect(removed).toBe(true)
  })
})

describe('public links', () => {
  test('creates a link with an expiry and copies it', async () => {
    const user = userEvent.setup()
    const state = setup()
    let body: unknown = null
    server.use(
      http.post(`/api/v1/nodes/${state.node.id}/links`, async ({ request }) => {
        body = await request.json()
        const created = link({ expires_at: '2026-10-06T10:00:00Z' })
        state.shares = { users: [], links: [created] }
        return HttpResponse.json(created, { status: 201 })
      }),
    )
    renderApp('/')

    await user.click(await screen.findByRole('button', { name: 'Actions for docs' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Share' }))
    const dialog = await screen.findByRole('dialog')
    await user.selectOptions(within(dialog).getByLabelText('Link expires'), '604800')
    await user.click(within(dialog).getByRole('button', { name: 'Create link' }))

    const url = `${window.location.origin}/s/tok123`
    expect(await within(dialog).findByDisplayValue(url)).toBeInTheDocument()
    expect(body).toEqual({ expires_in_seconds: 604800 })
    expect(within(dialog).getByText(/^Expires/)).toBeInTheDocument()

    await user.click(within(dialog).getByRole('button', { name: 'Copy link' }))
    expect(await navigator.clipboard.readText()).toBe(url)
    expect(await screen.findByText('Link copied.')).toBeInTheDocument()
  })

  test('creates a link that never expires by default', async () => {
    const state = setup()
    let body: unknown = null
    server.use(
      http.post(`/api/v1/nodes/${state.node.id}/links`, async ({ request }) => {
        body = await request.json()
        state.shares = { users: [], links: [link()] }
        return HttpResponse.json(link(), { status: 201 })
      }),
    )
    renderApp('/')

    const dialog = await openShare()
    await userEvent.click(within(dialog).getByRole('button', { name: 'Create link' }))

    expect(await within(dialog).findByText('Never expires')).toBeInTheDocument()
    expect(body).toEqual({})
  })

  test('revokes a link', async () => {
    const state = setup({ users: [], links: [link()] })
    let revoked = false
    server.use(
      http.delete(`/api/v1/nodes/${state.node.id}/links/l1`, () => {
        revoked = true
        state.shares = { users: [], links: [] }
        return new HttpResponse(null, { status: 204 })
      }),
    )
    renderApp('/')

    const dialog = await openShare()
    await userEvent.click(await within(dialog).findByRole('button', { name: 'Revoke link' }))

    await waitFor(() =>
      expect(within(dialog).queryByRole('button', { name: 'Revoke link' })).not.toBeInTheDocument(),
    )
    expect(revoked).toBe(true)
  })
})

test('only owners see Share', async () => {
  signedIn()
  const shared = folder('shared')
  mockFolder(shared, { role: 'editor', children: [file('x.txt', { parent_id: shared.id })] })
  renderApp(`/folders/${shared.id}`)

  await screen.findByRole('heading', { name: 'shared' })
  await userEvent.click(await screen.findByRole('button', { name: 'Actions for x.txt' }))
  expect(await screen.findByRole('menuitem', { name: 'Rename' })).toBeInTheDocument()
  expect(screen.queryByRole('menuitem', { name: 'Share' })).not.toBeInTheDocument()
})
