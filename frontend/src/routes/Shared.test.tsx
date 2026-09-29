import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { afterEach, expect, test, vi } from 'vitest'

import { signedIn } from '@/test/auth'
import { currentLocation, renderApp } from '@/test/app'
import { file, folder, mockFolder } from '@/test/nodes'
import { server } from '@/test/server'

afterEach(() => vi.restoreAllMocks())

test('lists nodes shared with me and opens a folder', async () => {
  signedIn()
  const project = folder('project', { parent_id: 'someone-elses-root' })
  server.use(http.get('/api/v1/shared', () => HttpResponse.json([file('notes.txt'), project])))
  mockFolder(project, { role: 'editor' })
  renderApp('/shared')

  expect(await screen.findByRole('heading', { name: 'Shared with me' })).toBeInTheDocument()
  await userEvent.click(await screen.findByRole('button', { name: 'project' }))

  await waitFor(() => expect(currentLocation()).toBe(`/folders/${project.id}`))
  expect(await screen.findByRole('heading', { name: 'project' })).toBeInTheDocument()
})

test('downloads a shared file', async () => {
  signedIn()
  const notes = file('notes.txt')
  server.use(
    http.get('/api/v1/shared', () => HttpResponse.json([notes])),
    http.get(`/api/v1/nodes/${notes.id}/download`, () =>
      HttpResponse.json({
        method: 'GET',
        url: 'https://storage.test/n?sig',
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
  renderApp('/shared')

  await userEvent.click(await screen.findByRole('button', { name: 'notes.txt' }))

  await waitFor(() => expect(clicked).toEqual(['https://storage.test/n?sig']))
})

test('shows an empty shared state', async () => {
  signedIn()
  server.use(http.get('/api/v1/shared', () => HttpResponse.json([])))
  renderApp('/shared')

  expect(await screen.findByText('Nothing is shared with you yet.')).toBeInTheDocument()
})
