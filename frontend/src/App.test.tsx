import { render, screen } from '@testing-library/react'
import { expect, test } from 'vitest'

import App from './App'

test('renders without crashing', () => {
  render(<App />)
  expect(screen.getByText('Trovr')).toBeInTheDocument()
})
