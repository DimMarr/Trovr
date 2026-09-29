import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, test } from 'vitest'

import { alice, signedIn } from '@/test/auth'
import { currentLocation, renderApp } from '@/test/app'
import { file, folder, mockFolder, mockMissingFolder, mockRoot } from '@/test/nodes'

function rowNames() {
  return within(screen.getByRole('table'))
    .getAllByRole('row')
    .slice(1)
    .map((row) => within(row).getAllByRole('cell')[0].textContent)
}

describe('browser', () => {
  test('lists the root with folders first, then by name', async () => {
    signedIn()
    mockRoot([file('b.txt'), folder('zeta'), file('a.txt'), folder('alpha')])
    renderApp('/')

    expect(await screen.findByRole('heading', { name: 'My files' })).toBeInTheDocument()
    await screen.findByText('alpha')
    expect(rowNames()).toEqual(['alpha', 'zeta', 'a.txt', 'b.txt'])
    expect(screen.getByText(alice.display_name)).toBeInTheDocument()
  })

  test('shows sizes for files only', async () => {
    signedIn()
    mockRoot([folder('docs'), file('notes.txt', { size_bytes: 1536 })])
    renderApp('/')

    const row = (await screen.findByText('notes.txt')).closest('tr')!
    expect(within(row).getByText('1.5 KB')).toBeInTheDocument()
    const folderRow = screen.getByText('docs').closest('tr')!
    expect(within(folderRow).getByText('—')).toBeInTheDocument()
  })

  test('opens a folder and shows its breadcrumb', async () => {
    signedIn()
    const docs = folder('docs')
    mockRoot([docs])
    mockFolder(docs, { children: [file('report.pdf', { parent_id: docs.id })] })
    renderApp('/')

    await userEvent.click(await screen.findByRole('button', { name: 'docs' }))

    expect(await screen.findByText('report.pdf')).toBeInTheDocument()
    expect(currentLocation()).toBe(`/folders/${docs.id}`)
    const crumbs = screen.getByRole('navigation', { name: 'Breadcrumb' })
    expect(within(crumbs).getByRole('link', { name: 'My files' })).toHaveAttribute('href', '/')
    expect(within(crumbs).getByText('docs')).toHaveAttribute('aria-current', 'page')
  })

  test('links every ancestor in the breadcrumb', async () => {
    signedIn()
    const a = folder('a')
    const b = folder('b', { parent_id: a.id })
    const c = folder('c', { parent_id: b.id })
    mockFolder(c, { path: [a, b, c] })
    renderApp(`/folders/${c.id}`)

    const crumbs = await screen.findByRole('navigation', { name: 'Breadcrumb' })
    expect(await within(crumbs).findByRole('link', { name: 'a' })).toHaveAttribute(
      'href',
      `/folders/${a.id}`,
    )
    expect(within(crumbs).getByRole('link', { name: 'b' })).toHaveAttribute(
      'href',
      `/folders/${b.id}`,
    )
  })

  test('starts a sharee breadcrumb at Shared with me', async () => {
    signedIn()
    const shared = folder('shared', { parent_id: 'hidden-parent' })
    mockFolder(shared, { role: 'viewer' })
    renderApp(`/folders/${shared.id}`)

    const crumbs = await screen.findByRole('navigation', { name: 'Breadcrumb' })
    expect(await within(crumbs).findByRole('link', { name: 'Shared with me' })).toHaveAttribute(
      'href',
      '/shared',
    )
  })

  test('shows an empty state', async () => {
    signedIn()
    mockRoot([])
    renderApp('/')

    expect(await screen.findByText('This folder is empty.')).toBeInTheDocument()
  })

  test('a missing folder shows the not-found state', async () => {
    signedIn()
    mockMissingFolder('gone')
    renderApp('/folders/gone')

    expect(await screen.findByText('Not found or no longer shared with you.')).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'Back to My files' })).toHaveAttribute('href', '/')
  })

  test('a folder whose access was revoked shows the not-found state', async () => {
    signedIn()
    mockMissingFolder('revoked', 403)
    renderApp('/folders/revoked')

    expect(await screen.findByText('Not found or no longer shared with you.')).toBeInTheDocument()
  })

  test('the sidebar links to every view', async () => {
    signedIn()
    mockRoot([])
    renderApp('/')

    const nav = await screen.findByRole('navigation', { name: 'Main' })
    expect(within(nav).getByRole('link', { name: 'My files' })).toHaveAttribute('href', '/')
    expect(within(nav).getByRole('link', { name: 'Shared with me' })).toHaveAttribute(
      'href',
      '/shared',
    )
    expect(within(nav).getByRole('link', { name: 'Trash' })).toHaveAttribute('href', '/trash')
  })
})
