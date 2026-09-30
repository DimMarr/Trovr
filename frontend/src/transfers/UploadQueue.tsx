import { useQueryClient } from '@tanstack/react-query'
import { type ReactNode, useCallback, useEffect, useMemo, useReducer, useRef } from 'react'

import { ApiError } from '@/api/client'
import { files } from '@/api/files'
import { nodes } from '@/api/nodes'
import type { Node } from '@/api/types'
import { useAuth } from '@/auth/context'
import { keys } from '@/hooks/nodes'
import { errorMessage } from '@/lib/errors'
import { formatBytes } from '@/lib/format'
import { strings } from '@/strings'

import { type UploadItem, type Uploads, UploadsContext } from './context'
import { uploadContent } from './upload'

const CONCURRENCY = 3

type Event =
  | { type: 'add'; items: UploadItem[] }
  | { type: 'update'; id: string; patch: Partial<UploadItem> }
  | { type: 'remove'; id: string }
  | { type: 'clearFinished' }

function reducer(items: UploadItem[], event: Event): UploadItem[] {
  switch (event.type) {
    case 'add':
      return [...items, ...event.items]
    case 'update':
      return items.map((item) => (item.id === event.id ? { ...item, ...event.patch } : item))
    case 'remove':
      return items.filter((item) => item.id !== event.id)
    case 'clearFinished':
      return items.filter((item) => item.status !== 'done')
  }
}

let nextId = 0

/** Uploads files in the background, three at a time, for the whole signed-in session. */
export function UploadQueue({ children }: { children: ReactNode }) {
  const { config } = useAuth()
  const queryClient = useQueryClient()
  const [items, dispatch] = useReducer(reducer, [])
  // Items already handed to a worker: effects may run twice for the same state.
  const started = useRef(new Set<string>())
  const maxBytes = config?.max_upload_bytes ?? Number.POSITIVE_INFINITY

  const update = useCallback(
    (id: string, patch: Partial<UploadItem>) => dispatch({ type: 'update', id, patch }),
    [],
  )

  const process = useCallback(
    async (item: UploadItem) => {
      let storageKey = item.storageKey
      try {
        if (!storageKey) {
          storageKey = await uploadContent(item.file, (progress) => update(item.id, { progress }))
        }
        update(item.id, { status: 'confirming', progress: 1, storageKey })
        await files.createFile(storageKey, item.parentId, item.file.name)
        update(item.id, { status: 'done' })
        await queryClient.invalidateQueries({ queryKey: keys.children(item.parentId) })
      } catch (error) {
        if (error instanceof ApiError && error.code === 'name_conflict') {
          update(item.id, { status: 'conflict', storageKey })
        } else {
          update(item.id, {
            status: 'error',
            error: errorMessage(error),
            retryable: true,
            storageKey,
          })
        }
      }
    },
    [queryClient, update],
  )

  useEffect(() => {
    const running = items.filter(
      (item) => item.status === 'uploading' || item.status === 'confirming',
    ).length
    const ready = items
      .filter((item) => item.status === 'queued' && !started.current.has(item.id))
      .slice(0, Math.max(0, CONCURRENCY - running))
    for (const item of ready) {
      started.current.add(item.id)
      update(item.id, { status: 'uploading' })
      void process(item)
    }
  }, [items, process, update])

  const replace = useCallback(
    async (id: string) => {
      const item = items.find((candidate) => candidate.id === id)
      if (!item?.storageKey) return
      update(id, { status: 'confirming' })
      try {
        const siblings =
          queryClient.getQueryData<Node[]>(keys.children(item.parentId)) ??
          (await (item.parentId === null ? nodes.listRoot() : nodes.children(item.parentId)))
        const existing = siblings.find(
          (node) => node.type === 'file' && node.name === item.file.name,
        )
        if (!existing) throw new ApiError(409, 'name_conflict', 'not a file')
        await files.createVersion(existing.id, item.storageKey)
        update(id, { status: 'done' })
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: keys.children(item.parentId) }),
          queryClient.invalidateQueries({ queryKey: ['versions', existing.id] }),
        ])
      } catch (error) {
        update(id, { status: 'error', error: errorMessage(error), retryable: false })
      }
    },
    [items, queryClient, update],
  )

  const value = useMemo<Uploads>(
    () => ({
      items,
      enqueue(selected, parentId) {
        const added = selected.map<UploadItem>((file) => {
          nextId += 1
          const tooLarge = file.size > maxBytes
          return {
            id: `upload-${nextId}`,
            file,
            parentId,
            status: tooLarge ? 'error' : 'queued',
            progress: 0,
            error: tooLarge ? strings.uploads.tooLarge(formatBytes(maxBytes)) : undefined,
            retryable: !tooLarge,
          }
        })
        dispatch({ type: 'add', items: added })
      },
      retry(id) {
        started.current.delete(id)
        update(id, { status: 'queued', error: undefined, progress: 0 })
      },
      replace: (id) => void replace(id),
      dismiss: (id) => dispatch({ type: 'remove', id }),
      clearFinished: () => dispatch({ type: 'clearFinished' }),
    }),
    [items, maxBytes, replace, update],
  )

  return <UploadsContext.Provider value={value}>{children}</UploadsContext.Provider>
}
