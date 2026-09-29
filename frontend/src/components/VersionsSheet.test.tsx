import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { afterEach, describe, expect, test, vi } from 'vitest'

import type { Node, Version } from '@/api/types'
import { signedIn } from '@/test/auth'
import { renderApp } from '@/test/app'
import { file, folder, mockFolder } from '@/test/nodes'
import { server } from '@/test/server'
import { mockUploads } from '@/test/uploads'

function version(number: number): Version {
  return {
    id: `v${number}`,
    version_number: number,
    size_bytes: 100 * number,
    checksum_sha256: null,
    created_by: 'u-alice',
    created_at: `2026-09-0${number}T10:00:00Z`,
  }
}

function mockVersions(node: Node, initial: Version[]) {
  const state = { versions: initial }
  server.use(http.get(`/api/v1/nodes/${node.id}/versions`, () => HttpResponse.json(state.versions)))
  return state
}

function captureDownloads() {
  const clicked: string[] = []
  vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
    this: HTMLAnchorElement,
  ) {
    clicked.push(this.href)
  })
  return clicked
}

async function openVersions(name: string) {
  await userEvent.click(await screen.findByRole('button', { name: `Actions for ${name}` }))
  await userEvent.click(await screen.findByRole('menuitem', { name: 'Versions' }))
  return screen.findByRole('dialog')
}

afterEach(() => vi.restoreAllMocks())

describe('versions', () => {
  test('lists versions newest first', async () => {
    signedIn()
    const report = file('report.txt')
    server.use(http.get('/api/v1/nodes', () => HttpResponse.json([report])))
    mockVersions(report, [version(1), version(3), version(2)])
    renderApp('/')

    const sheet = await openVersions('report.txt')
    const items = await within(sheet).findAllByRole('listitem')
    expect(items.map((item) => within(item).getByText(/^Version \d$/).textContent)).toEqual([
      'Version 3',
      'Version 2',
      'Version 1',
    ])
    expect(within(items[0]).getByText('Current')).toBeInTheDocument()
  })

  test('downloads a specific version', async () => {
    signedIn()
    const report = file('report.txt')
    server.use(http.get('/api/v1/nodes', () => HttpResponse.json([report])))
    mockVersions(report, [version(1), version(2)])
    let requested: string | null = null
    server.use(
      http.get(`/api/v1/nodes/${report.id}/download`, ({ request }) => {
        requested = new URL(request.url).searchParams.get('version_id')
        return HttpResponse.json({
          method: 'GET',
          url: 'https://storage.test/trovr/old?sig',
          headers: {},
          expires_at: '',
        })
      }),
    )
    const clicked = captureDownloads()
    renderApp('/')

    const sheet = await openVersions('report.txt')
    await userEvent.click(await within(sheet).findByRole('button', { name: 'Download version 1' }))

    await waitFor(() => expect(clicked).toEqual(['https://storage.test/trovr/old?sig']))
    expect(requested).toBe('v1')
  })

  test('uploads a new version', async () => {
    signedIn()
    const report = file('report.txt')
    server.use(http.get('/api/v1/nodes', () => HttpResponse.json([report])))
    const state = mockVersions(report, [version(1)])
    const log = mockUploads()
    let body: unknown = null
    server.use(
      http.post(`/api/v1/nodes/${report.id}/versions`, async ({ request }) => {
        body = await request.json()
        state.versions = [version(1), version(2)]
        return HttpResponse.json(version(2), { status: 201 })
      }),
    )
    renderApp('/')

    const sheet = await openVersions('report.txt')
    await userEvent.upload(
      within(sheet).getByLabelText('Upload a new version'),
      new File(['v2'], 'report.txt', { type: 'text/plain' }),
    )

    expect(await within(sheet).findByText('Version 2')).toBeInTheDocument()
    expect(log.steps).toEqual(['presign', 'put'])
    expect(body).toEqual({ storage_key: 'users/u-alice/key-1' })
  })

  test('viewers can see versions but not upload one', async () => {
    signedIn()
    const shared = folder('shared')
    const report = file('report.txt', { parent_id: shared.id })
    mockFolder(shared, { role: 'viewer', children: [report] })
    mockVersions(report, [version(1)])
    renderApp(`/folders/${shared.id}`)

    await screen.findByRole('heading', { name: 'shared' })
    const sheet = await openVersions('report.txt')
    expect(await within(sheet).findByText('Version 1')).toBeInTheDocument()
    expect(within(sheet).queryByLabelText('Upload a new version')).not.toBeInTheDocument()
  })

  test('folders have no versions entry', async () => {
    signedIn()
    const docs = folder('docs')
    server.use(http.get('/api/v1/nodes', () => HttpResponse.json([docs])))
    renderApp('/')

    await userEvent.click(await screen.findByRole('button', { name: 'Actions for docs' }))
    expect(await screen.findByRole('menuitem', { name: 'Rename' })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Versions' })).not.toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Download' })).not.toBeInTheDocument()
  })

  test('the row menu downloads a file', async () => {
    signedIn()
    const report = file('report.txt')
    server.use(
      http.get('/api/v1/nodes', () => HttpResponse.json([report])),
      http.get(`/api/v1/nodes/${report.id}/download`, () =>
        HttpResponse.json({
          method: 'GET',
          url: 'https://storage.test/r?sig',
          headers: {},
          expires_at: '',
        }),
      ),
    )
    const clicked = captureDownloads()
    renderApp('/')

    await userEvent.click(await screen.findByRole('button', { name: 'Actions for report.txt' }))
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Download' }))

    await waitFor(() => expect(clicked).toEqual(['https://storage.test/r?sig']))
  })
})
