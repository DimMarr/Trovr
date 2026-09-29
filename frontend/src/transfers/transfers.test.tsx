import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { afterEach, describe, expect, test, vi } from 'vitest'

import type { Node } from '@/api/types'
import { mockAuthConfig, mockMe, storeSession } from '@/test/auth'
import { renderApp } from '@/test/app'
import { file, folder, mockFolder } from '@/test/nodes'
import { server } from '@/test/server'
import { mockUploads } from '@/test/uploads'

function signedInWithLimit(maxBytes = 1024 * 1024) {
  mockAuthConfig({ max_upload_bytes: maxBytes })
  mockMe()
  storeSession()
}

/** Serves the root listing from a mutable list. */
function liveRoot(initial: Node[] = []) {
  const root = { children: initial }
  server.use(http.get('/api/v1/nodes', () => HttpResponse.json(root.children)))
  return root
}

async function pickFiles(...files: File[]) {
  await userEvent.upload(await screen.findByLabelText('Upload files'), files)
}

const uploads = () => screen.findByRole('region', { name: 'Uploads' })

afterEach(() => vi.restoreAllMocks())

describe('uploads', () => {
  test('uploads by presign, put, and confirm in order', async () => {
    signedInWithLimit()
    const root = liveRoot()
    const log = mockUploads({
      onConfirm: (body) => {
        const created = file(body.name)
        root.children = [created]
        return created
      },
    })
    renderApp('/')

    await pickFiles(new File(['hello'], 'hello.txt', { type: 'text/plain' }))

    expect(await screen.findByRole('button', { name: 'hello.txt' })).toBeInTheDocument()
    expect(log.steps).toEqual(['presign', 'put', 'confirm'])
    expect(log.presigned).toEqual([{ size_bytes: 5, mime_type: 'text/plain' }])
    expect(log.putHeaders[0]['content-type']).toBe('text/plain')
    expect(log.confirmed).toEqual([
      { storage_key: 'users/u-alice/key-1', parent_id: null, name: 'hello.txt' },
    ])
    expect(await within(await uploads()).findByText('Uploaded')).toBeInTheDocument()
  })

  test('refuses files over the size limit before any request', async () => {
    signedInWithLimit(4)
    liveRoot()
    const log = mockUploads()
    renderApp('/')

    await pickFiles(new File(['too large'], 'big.bin'))

    expect(
      await within(await uploads()).findByText('Larger than the 4 B limit.'),
    ).toBeInTheDocument()
    expect(log.steps).toEqual([])
  })

  test('empty files and unknown types upload', async () => {
    signedInWithLimit()
    liveRoot()
    const log = mockUploads()
    renderApp('/')

    await pickFiles(new File([], 'empty', { type: '' }))

    await waitFor(() => expect(log.steps).toEqual(['presign', 'put', 'confirm']))
    expect(log.presigned).toEqual([{ size_bytes: 0, mime_type: 'application/octet-stream' }])
  })

  test('runs at most three uploads at a time', async () => {
    signedInWithLimit()
    liveRoot()
    const log = mockUploads({ holdPuts: true })
    renderApp('/')

    await pickFiles(...['1', '2', '3', '4', '5'].map((n) => new File([n], `${n}.txt`)))

    await waitFor(() => expect(log.activePuts).toBe(3))
    // Give the queue a chance to (wrongly) start a fourth one.
    await new Promise((resolve) => setTimeout(resolve, 50))
    expect(log.presigned).toHaveLength(3)

    log.release()
    await waitFor(() => expect(log.confirmed).toHaveLength(5))
    expect(log.maxConcurrentPuts).toBe(3)
  })

  test('a name conflict offers replace, which adds a version', async () => {
    signedInWithLimit()
    const existing = file('report.txt')
    liveRoot([existing])
    mockUploads()
    let version: unknown = null
    server.use(
      http.post('/api/v1/files', () =>
        HttpResponse.json(
          { error: { code: 'name_conflict', message: 'a node with this name already exists' } },
          { status: 409 },
        ),
      ),
      http.post(`/api/v1/nodes/${existing.id}/versions`, async ({ request }) => {
        version = await request.json()
        return HttpResponse.json({ id: 'v2', version_number: 2 }, { status: 201 })
      }),
    )
    renderApp('/')
    await screen.findByRole('button', { name: 'report.txt' })

    await pickFiles(new File(['v2'], 'report.txt'))
    const panel = await uploads()
    expect(
      await within(panel).findByText('A file with this name already exists.'),
    ).toBeInTheDocument()
    await userEvent.click(within(panel).getByRole('button', { name: 'Replace' }))

    expect(await within(panel).findByText('Uploaded')).toBeInTheDocument()
    expect(version).toEqual({ storage_key: 'users/u-alice/key-1' })
  })

  test('failed uploads can be retried', async () => {
    signedInWithLimit()
    liveRoot()
    const log = mockUploads()
    let failures = 1
    server.use(
      http.post('/api/v1/files', () => {
        if (failures-- > 0) {
          return HttpResponse.json(
            { error: { code: 'storage_unavailable', message: 'down' } },
            { status: 502 },
          )
        }
        log.steps.push('confirm')
        return HttpResponse.json(file('a.txt'), { status: 201 })
      }),
    )
    renderApp('/')

    await pickFiles(new File(['a'], 'a.txt'))
    const panel = await uploads()
    await userEvent.click(await within(panel).findByRole('button', { name: 'Retry' }))

    expect(await within(panel).findByText('Uploaded')).toBeInTheDocument()
    expect(log.steps.filter((step) => step === 'put')).toHaveLength(1)
  })

  test('dropping files onto the browser enqueues them', async () => {
    signedInWithLimit()
    const docs = folder('docs')
    mockFolder(docs, { role: 'editor' })
    const log = mockUploads()
    renderApp(`/folders/${docs.id}`)
    await screen.findByRole('heading', { name: 'docs' })

    const zone = screen.getByTestId('drop-zone')
    fireEvent.drop(zone, {
      dataTransfer: { files: [new File(['x'], 'dropped.txt')], types: ['Files'] },
    })

    await waitFor(() => expect(log.confirmed).toHaveLength(1))
    expect(log.confirmed[0]).toMatchObject({ parent_id: docs.id, name: 'dropped.txt' })
  })

  test('viewers cannot upload by button or drop', async () => {
    signedInWithLimit()
    const docs = folder('docs')
    mockFolder(docs, { role: 'viewer', children: [file('x.txt', { parent_id: docs.id })] })
    const log = mockUploads()
    renderApp(`/folders/${docs.id}`)

    await screen.findByRole('heading', { name: 'docs' })
    await screen.findByRole('button', { name: 'x.txt' })
    expect(screen.queryByLabelText('Upload files')).not.toBeInTheDocument()
    fireEvent.drop(screen.getByTestId('drop-zone'), {
      dataTransfer: { files: [new File(['x'], 'dropped.txt')], types: ['Files'] },
    })
    await new Promise((resolve) => setTimeout(resolve, 20))
    expect(log.steps).toEqual([])
  })
})

describe('downloads', () => {
  test('clicking a file starts its download', async () => {
    signedInWithLimit()
    const report = file('report.pdf')
    liveRoot([report])
    server.use(
      http.get(`/api/v1/nodes/${report.id}/download`, () =>
        HttpResponse.json({
          method: 'GET',
          url: 'https://storage.test/trovr/report?X-Amz-Signature=sig',
          headers: {},
          expires_at: '2026-09-29T12:00:00Z',
        }),
      ),
    )
    const clicked: string[] = []
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      clicked.push(this.href)
    })
    renderApp('/')

    await userEvent.click(await screen.findByRole('button', { name: 'report.pdf' }))

    await waitFor(() =>
      expect(clicked).toEqual(['https://storage.test/trovr/report?X-Amz-Signature=sig']),
    )
  })
})
