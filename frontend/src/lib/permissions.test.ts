import { expect, test } from 'vitest'

import { type Action, can } from './permissions'

const actions: Action[] = [
  'download',
  'versions',
  'createFolder',
  'upload',
  'rename',
  'move',
  'trash',
  'addVersion',
  'share',
]

test.each([
  ['viewer', ['download', 'versions']],
  [
    'editor',
    ['download', 'versions', 'createFolder', 'upload', 'rename', 'move', 'trash', 'addVersion'],
  ],
  ['owner', actions],
] as const)('%s may %j', (role, allowed) => {
  expect(actions.filter((action) => can(role, action))).toEqual(allowed)
})

test('nothing is allowed without a role', () => {
  expect(actions.some((action) => can(undefined, action))).toBe(false)
})
