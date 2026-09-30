import { render, screen } from '@testing-library/react'
import { expect, test } from 'vitest'

import { mockAuthConfig } from '@/test/auth'

import App from './App'

test('renders the sign-in page for anonymous visitors', async () => {
  mockAuthConfig()
  render(<App />)
  expect(await screen.findByText('Sign in to Trovr')).toBeInTheDocument()
})
