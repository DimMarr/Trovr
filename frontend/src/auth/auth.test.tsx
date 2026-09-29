import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { describe, expect, test } from 'vitest'

import { request } from '@/api/client'
import { currentLocation, renderApp } from '@/test/app'
import { alice, fakeJwt, fakeOidcClient, mockAuthConfig, mockMe, storeSession } from '@/test/auth'
import { server } from '@/test/server'

const location = currentLocation

describe('internal sign-in', () => {
  test('signs in with email and password and goes to next', async () => {
    mockAuthConfig()
    mockMe()
    let credentials: unknown = null
    server.use(
      http.post('/api/v1/auth/login', async ({ request }) => {
        credentials = await request.json()
        return HttpResponse.json({ access_token: fakeJwt(), token_type: 'Bearer' })
      }),
    )
    renderApp('/login?next=%2Ffolders%2Fabc')

    await userEvent.type(await screen.findByLabelText('Email'), 'alice@example.com')
    await userEvent.type(screen.getByLabelText('Password'), 'correct horse')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))

    await waitFor(() => expect(location()).toBe('/folders/abc'))
    expect(credentials).toEqual({ email: 'alice@example.com', password: 'correct horse' })
    expect(JSON.parse(sessionStorage.getItem('trovr.session')!)).toMatchObject({
      method: 'internal',
    })
  })

  test('shows invalid credentials and stays on the page', async () => {
    mockAuthConfig()
    server.use(
      http.post('/api/v1/auth/login', () =>
        HttpResponse.json(
          { error: { code: 'invalid_credentials', message: 'invalid credentials' } },
          { status: 401 },
        ),
      ),
    )
    renderApp('/login')

    await userEvent.type(await screen.findByLabelText('Email'), 'alice@example.com')
    await userEvent.type(screen.getByLabelText('Password'), 'wrong password')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))

    expect(await screen.findByText('Wrong email or password.')).toBeInTheDocument()
    expect(location()).toBe('/login')
  })

  test('never redirects outside the app after sign-in', async () => {
    mockAuthConfig()
    mockMe()
    server.use(
      http.post('/api/v1/auth/login', () => HttpResponse.json({ access_token: fakeJwt() })),
    )
    renderApp('/login?next=%2F%2Fevil.example.com')

    await userEvent.type(await screen.findByLabelText('Email'), 'alice@example.com')
    await userEvent.type(screen.getByLabelText('Password'), 'correct horse')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))

    await waitFor(() => expect(location()).toBe('/'))
  })

  test('registers then signs in when registration is enabled', async () => {
    mockAuthConfig({ internal: { enabled: true, registration: true } })
    mockMe()
    let registered: unknown = null
    server.use(
      http.post('/api/v1/auth/register', async ({ request }) => {
        registered = await request.json()
        return HttpResponse.json(alice, { status: 201 })
      }),
      http.post('/api/v1/auth/login', () => HttpResponse.json({ access_token: fakeJwt() })),
    )
    renderApp('/login')

    await userEvent.click(await screen.findByRole('link', { name: 'Create an account' }))
    await userEvent.type(await screen.findByLabelText('Name'), 'Alice')
    await userEvent.type(screen.getByLabelText('Email'), 'alice@example.com')
    await userEvent.type(screen.getByLabelText('Password'), 'correct horse')
    await userEvent.click(screen.getByRole('button', { name: 'Create account' }))

    await waitFor(() => expect(location()).toBe('/'))
    expect(registered).toEqual({
      email: 'alice@example.com',
      password: 'correct horse',
      display_name: 'Alice',
    })
  })

  test('hides register when registration is disabled', async () => {
    mockAuthConfig({ internal: { enabled: true, registration: false } })
    renderApp('/login')

    expect(await screen.findByRole('button', { name: 'Sign in' })).toBeInTheDocument()
    expect(screen.queryByRole('link', { name: 'Create an account' })).not.toBeInTheDocument()
  })
})

describe('OIDC sign-in', () => {
  test('shows the SSO button only when OIDC is configured', async () => {
    mockAuthConfig({
      internal: { enabled: false, registration: false },
      oidc: { issuer: 'https://idp.example.com', client_id: 'trovr' },
    })
    renderApp('/login')

    expect(await screen.findByRole('button', { name: 'Sign in with SSO' })).toBeInTheDocument()
    expect(screen.queryByLabelText('Email')).not.toBeInTheDocument()
  })

  test('has no SSO button without OIDC', async () => {
    mockAuthConfig()
    renderApp('/login')

    expect(await screen.findByLabelText('Email')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sign in with SSO' })).not.toBeInTheDocument()
  })

  test('SSO sign-in delegates to the OIDC client with next', async () => {
    mockAuthConfig({ oidc: { issuer: 'https://idp.example.com', client_id: 'trovr' } })
    const oidc = fakeOidcClient()
    renderApp('/login?next=%2Fshared', oidc)

    await userEvent.click(await screen.findByRole('button', { name: 'Sign in with SSO' }))

    expect(oidc.signIn).toHaveBeenCalledWith('/shared')
  })

  test('the callback completes sign-in and goes to next', async () => {
    mockAuthConfig({ oidc: { issuer: 'https://idp.example.com', client_id: 'trovr' } })
    mockMe()
    const oidc = fakeOidcClient({
      completeSignIn: async () => ({ accessToken: fakeJwt(), next: '/trash' }),
    })
    renderApp('/auth/callback?code=x&state=y', oidc)

    await waitFor(() => expect(location()).toBe('/trash'))
    expect(JSON.parse(sessionStorage.getItem('trovr.session')!)).toMatchObject({ method: 'oidc' })
  })
})

describe('session', () => {
  test('redirects anonymous visitors to login with next', async () => {
    mockAuthConfig()
    renderApp('/folders/abc')

    await waitFor(() => expect(location()).toBe('/login?next=%2Ffolders%2Fabc'))
  })

  test('restores the session from sessionStorage on reload', async () => {
    mockAuthConfig()
    mockMe()
    storeSession()
    renderApp('/folders/abc')

    expect(await screen.findByText(alice.display_name)).toBeInTheDocument()
    expect(location()).toBe('/folders/abc')
  })

  test('ignores an expired stored session', async () => {
    mockAuthConfig()
    storeSession(fakeJwt(-10))
    renderApp('/folders/abc')

    await waitFor(() => expect(location()).toBe('/login?next=%2Ffolders%2Fabc'))
    expect(sessionStorage.getItem('trovr.session')).toBeNull()
  })

  test('an expired session redirects to login with next', async () => {
    mockAuthConfig()
    mockMe()
    storeSession()
    renderApp('/folders/abc')
    await screen.findByText(alice.display_name)

    server.use(
      http.get('/api/v1/nodes', () =>
        HttpResponse.json({ error: { code: 'unauthorized', message: 'no' } }, { status: 401 }),
      ),
    )
    await request('GET', '/nodes').catch(() => {})

    await waitFor(() => expect(location()).toBe('/login?next=%2Ffolders%2Fabc'))
    expect(sessionStorage.getItem('trovr.session')).toBeNull()
  })

  test('logout clears the session', async () => {
    mockAuthConfig()
    mockMe()
    storeSession()
    renderApp('/')

    await userEvent.click(await screen.findByRole('button', { name: 'Sign out' }))

    await waitFor(() => expect(location()).toMatch(/^\/login/))
    expect(sessionStorage.getItem('trovr.session')).toBeNull()
  })

  test('logout from an OIDC session signs out at the identity provider', async () => {
    mockAuthConfig({ oidc: { issuer: 'https://idp.example.com', client_id: 'trovr' } })
    mockMe()
    storeSession(fakeJwt(), 'oidc')
    const oidc = fakeOidcClient()
    renderApp('/', oidc)

    await userEvent.click(await screen.findByRole('button', { name: 'Sign out' }))

    await waitFor(() => expect(oidc.signOut).toHaveBeenCalled())
    expect(sessionStorage.getItem('trovr.session')).toBeNull()
  })
})
