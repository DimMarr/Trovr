import { expect, test } from 'vitest'

import { formatBytes } from './format'

test.each([
  [0, '0 B'],
  [512, '512 B'],
  [1536, '1.5 KB'],
  [1024 * 1024, '1 MB'],
  [5 * 1024 ** 3, '5 GB'],
])('formatBytes(%d) is %s', (bytes, expected) => {
  expect(formatBytes(bytes)).toBe(expected)
})
