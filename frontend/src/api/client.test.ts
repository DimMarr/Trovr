import { http, HttpResponse } from 'msw'
import { afterEach, describe, expect, test, vi } from 'vitest'

import { server } from '@/test/server'

import { ApiError, configureApi, request } from './client'
import { files } from './files'

function signedIn(token: string | null, onUnauthorized = vi.fn()) {
  configureApi({ getToken: () => token, onUnauthorized })
  return onUnauthorized
}

afterEach(() => signedIn(null))

describe('request', () => {
  test('sends the bearer token when signed in', async () => {
    signedIn('abc')
    let authorization: string | null = null
    server.use(
      http.get('/api/v1/me', ({ request }) => {
        authorization = request.headers.get('authorization')
        return HttpResponse.json({ id: '1' })
      }),
    )

    await expect(request('GET', '/me')).resolves.toEqual({ id: '1' })
    expect(authorization).toBe('Bearer abc')
  })

  test('omits authorization for public calls', async () => {
    signedIn('abc')
    let authorization: string | null = 'unset'
    server.use(
      http.get('/api/v1/auth/config', ({ request }) => {
        authorization = request.headers.get('authorization')
        return HttpResponse.json({})
      }),
    )

    await request('GET', '/auth/config', { auth: false })
    expect(authorization).toBeNull()
  })

  test('sends JSON bodies', async () => {
    let received: unknown = null
    server.use(
      http.post('/api/v1/folders', async ({ request }) => {
        received = await request.json()
        return HttpResponse.json({ id: 'f' }, { status: 201 })
      }),
    )

    await request('POST', '/folders', { body: { name: 'docs', parent_id: null } })
    expect(received).toEqual({ name: 'docs', parent_id: null })
  })

  test('turns error bodies into ApiError', async () => {
    server.use(
      http.post('/api/v1/folders', () =>
        HttpResponse.json(
          { error: { code: 'name_conflict', message: 'a node with this name already exists' } },
          { status: 409 },
        ),
      ),
    )

    const error = await request('POST', '/folders', { body: {} }).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect(error).toMatchObject({
      status: 409,
      code: 'name_conflict',
      message: 'a node with this name already exists',
    })
  })

  test('keeps the status when the error body is not JSON', async () => {
    server.use(http.get('/api/v1/me', () => new HttpResponse('bad gateway', { status: 502 })))

    await expect(request('GET', '/me')).rejects.toMatchObject({ status: 502, code: 'http_502' })
  })

  test('reports network failures as network_error', async () => {
    server.use(http.get('/api/v1/me', () => HttpResponse.error()))

    await expect(request('GET', '/me')).rejects.toMatchObject({ status: 0, code: 'network_error' })
  })

  test('calls onUnauthorized on 401 for authenticated calls only', async () => {
    const onUnauthorized = signedIn('expired')
    const unauthorized = () =>
      HttpResponse.json({ error: { code: 'unauthorized', message: 'no' } }, { status: 401 })
    server.use(http.get('/api/v1/me', unauthorized), http.post('/api/v1/auth/login', unauthorized))

    await expect(request('GET', '/me')).rejects.toMatchObject({ status: 401 })
    expect(onUnauthorized).toHaveBeenCalledTimes(1)

    await expect(request('POST', '/auth/login', { auth: false, body: {} })).rejects.toBeInstanceOf(
      ApiError,
    )
    expect(onUnauthorized).toHaveBeenCalledTimes(1)
  })

  test('returns undefined for 204', async () => {
    server.use(http.delete('/api/v1/trash/n', () => new HttpResponse(null, { status: 204 })))

    await expect(request('DELETE', '/trash/n')).resolves.toBeUndefined()
  })
})

test('download asks for a specific version when given one', async () => {
  let url = ''
  server.use(
    http.get('/api/v1/nodes/:id/download', ({ request }) => {
      url = request.url
      return HttpResponse.json({ method: 'GET', url: 'https://s3/x', headers: {}, expires_at: '' })
    }),
  )

  await files.download('n1', 'v2')
  expect(new URL(url).pathname).toBe('/api/v1/nodes/n1/download')
  expect(new URL(url).searchParams.get('version_id')).toBe('v2')
})
