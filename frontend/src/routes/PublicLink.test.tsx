import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { afterEach, describe, expect, test, vi } from 'vitest'

import type { Node } from '@/api/types'
import { mockAuthConfig } from '@/test/auth'
import { renderApp } from '@/test/app'
import { file, folder } from '@/test/nodes'
import { server } from '@/test/server'

const TOKEN = 'tok123'

/** A public link to `root`, whose folders' children are given by `tree`. */
function mockLink(root: Node, tree: Record<string, Node[]> = {}) {
  const authorization: (string | null)[] = []
  server.use(
    http.get(`/api/v1/public/${TOKEN}`, ({ request }) => {
      authorization.push(request.headers.get('authorization'))
      return HttpResponse.json(root)
    }),
    http.get(`/api/v1/public/${TOKEN}/nodes/:id/children`, ({ request, params }) => {
      authorization.push(request.headers.get('authorization'))
      const children = tree[params.id as string]
      return children
        ? HttpResponse.json(children)
        : HttpResponse.json({ error: { code: 'not_found', message: 'nope' } }, { status: 404 })
    }),
  )
  return authorization
}

afterEach(() => vi.restoreAllMocks())

describe('public link page', () => {
  test('browses a public folder without signing in', async () => {
    mockAuthConfig()
    const holiday = folder('Holiday')
    const day1 = folder('day1', { parent_id: holiday.id })
    const authorization = mockLink(holiday, {
      [holiday.id]: [file('a.jpg', { parent_id: holiday.id }), day1],
      [day1.id]: [file('beach.jpg', { parent_id: day1.id })],
    })
    renderApp(`/s/${TOKEN}`)

    expect(await screen.findByRole('heading', { name: 'Holiday' })).toBeInTheDocument()
    await userEvent.click(await screen.findByRole('button', { name: 'day1' }))

    expect(await screen.findByRole('button', { name: 'beach.jpg' })).toBeInTheDocument()
    const crumbs = screen.getByRole('navigation', { name: 'Breadcrumb' })
    expect(within(crumbs).getByRole('link', { name: 'Holiday' })).toHaveAttribute(
      'href',
      `/s/${TOKEN}`,
    )
    expect(within(crumbs).getByText('day1')).toHaveAttribute('aria-current', 'page')
    expect(authorization.every((header) => header === null)).toBe(true)
  })

  test('downloads a public file', async () => {
    mockAuthConfig()
    const holiday = folder('Holiday')
    const photo = file('a.jpg', { parent_id: holiday.id })
    mockLink(holiday, { [holiday.id]: [photo] })
    server.use(
      http.get(`/api/v1/public/${TOKEN}/nodes/${photo.id}/download`, () =>
        HttpResponse.json({
          method: 'GET',
          url: 'https://storage.test/a?sig',
          headers: {},
          expires_at: '',
        }),
      ),
    )
    const clicked: string[] = []
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      clicked.push(this.href)
    })
    renderApp(`/s/${TOKEN}`)

    await userEvent.click(await screen.findByRole('button', { name: 'a.jpg' }))

    await waitFor(() => expect(clicked).toEqual(['https://storage.test/a?sig']))
  })

  test('a link to a single file offers its download', async () => {
    mockAuthConfig()
    const report = file('report.pdf')
    mockLink(report)
    renderApp(`/s/${TOKEN}`)

    expect(await screen.findByRole('heading', { name: 'report.pdf' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Download' })).toBeInTheDocument()
  })

  test('a dead link shows it is no longer valid', async () => {
    mockAuthConfig()
    server.use(
      http.get(`/api/v1/public/${TOKEN}`, () =>
        HttpResponse.json({ error: { code: 'not_found', message: 'nope' } }, { status: 404 }),
      ),
    )
    renderApp(`/s/${TOKEN}`)

    expect(await screen.findByText('This link is no longer valid.')).toBeInTheDocument()
  })

  test('a reload inside a subfolder keeps browsing', async () => {
    mockAuthConfig()
    const holiday = folder('Holiday')
    const day1 = folder('day1', { parent_id: holiday.id })
    mockLink(holiday, { [day1.id]: [file('beach.jpg', { parent_id: day1.id })] })
    renderApp(`/s/${TOKEN}/${day1.id}`)

    expect(await screen.findByRole('button', { name: 'beach.jpg' })).toBeInTheDocument()
    const crumbs = screen.getByRole('navigation', { name: 'Breadcrumb' })
    expect(within(crumbs).getByRole('link', { name: 'Holiday' })).toBeInTheDocument()
    expect(within(crumbs).getByText('…')).toBeInTheDocument()
  })

  test('a folder outside the link shows not found', async () => {
    mockAuthConfig()
    const holiday = folder('Holiday')
    mockLink(holiday, { [holiday.id]: [] })
    renderApp(`/s/${TOKEN}/somewhere-else`)

    expect(
      await screen.findByText('This folder is not part of the shared link.'),
    ).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Back to the shared folder' })).toHaveAttribute(
      'href',
      `/s/${TOKEN}`,
    )
  })
})
