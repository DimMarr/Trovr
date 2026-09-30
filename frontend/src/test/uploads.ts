import { http, HttpResponse } from 'msw'

import type { Node } from '@/api/types'

import { file } from './nodes'
import { server } from './server'

export const STORAGE = 'https://storage.test/trovr'

/** What a bucket configured for the web origin answers (see frontend/README.md). */
const CORS = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'GET, PUT',
  'Access-Control-Allow-Headers': 'content-type',
}

export interface UploadLog {
  steps: string[]
  presigned: unknown[]
  putHeaders: Record<string, string>[]
  confirmed: unknown[]
  maxConcurrentPuts: number
  activePuts: number
  /** Lets held PUTs (see `holdPuts`) complete. */
  release: () => void
}

/** Mocks presign → PUT to storage → confirm; confirmations add the file to `root`. */
export function mockUploads({
  holdPuts = false,
  onConfirm,
}: {
  /** Keep every PUT pending until `log.release()`, to observe concurrency. */
  holdPuts?: boolean
  onConfirm?: (body: { name: string; parent_id: string | null }) => Node
} = {}) {
  let release = () => {}
  const released = holdPuts
    ? new Promise<void>((resolve) => (release = resolve))
    : Promise.resolve()
  const log: UploadLog = {
    steps: [],
    presigned: [],
    putHeaders: [],
    confirmed: [],
    maxConcurrentPuts: 0,
    activePuts: 0,
    release: () => release(),
  }
  let counter = 0
  server.use(
    http.post('/api/v1/uploads', async ({ request }) => {
      counter += 1
      log.steps.push('presign')
      log.presigned.push(await request.json())
      const key = `users/u-alice/key-${counter}`
      return HttpResponse.json(
        {
          storage_key: key,
          upload: {
            method: 'PUT',
            url: `${STORAGE}/${key}?X-Amz-Signature=sig`,
            headers: { 'content-type': 'text/plain', 'content-length': '5', host: 'storage.test' },
            expires_at: '2026-09-29T12:00:00Z',
          },
        },
        { status: 201 },
      )
    }),
    http.options(
      `${STORAGE}/users/:user/:key`,
      () => new HttpResponse(null, { status: 204, headers: CORS }),
    ),
    http.put(`${STORAGE}/users/:user/:key`, async ({ request }) => {
      log.steps.push('put')
      log.putHeaders.push(Object.fromEntries(request.headers.entries()))
      log.activePuts += 1
      log.maxConcurrentPuts = Math.max(log.maxConcurrentPuts, log.activePuts)
      await released
      log.activePuts -= 1
      return new HttpResponse(null, { status: 200, headers: CORS })
    }),
    http.post('/api/v1/files', async ({ request }) => {
      log.steps.push('confirm')
      const body = (await request.json()) as { name: string; parent_id: string | null }
      log.confirmed.push(body)
      const created = onConfirm?.(body) ?? file(body.name, { parent_id: body.parent_id })
      return HttpResponse.json(created, { status: 201 })
    }),
  )
  return log
}
