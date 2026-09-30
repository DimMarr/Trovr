import { createContext, useContext } from 'react'

export type UploadStatus = 'queued' | 'uploading' | 'confirming' | 'done' | 'error' | 'conflict'

export interface UploadItem {
  id: string
  file: File
  parentId: string | null
  status: UploadStatus
  /** 0 to 1, while uploading. */
  progress: number
  error?: string
  /** Whether retrying can help (not for files over the size limit). */
  retryable?: boolean
  /** Set once the bytes are in storage, so a retry or a replace does not upload again. */
  storageKey?: string
}

export interface Uploads {
  items: UploadItem[]
  enqueue(files: File[], parentId: string | null): void
  retry(id: string): void
  /** Resolves a name conflict by adding the upload as a new version of the existing file. */
  replace(id: string): void
  dismiss(id: string): void
  clearFinished(): void
}

export const UploadsContext = createContext<Uploads | null>(null)

export function useUploads(): Uploads {
  const uploads = useContext(UploadsContext)
  if (!uploads) throw new Error('useUploads must be used inside <UploadQueue>')
  return uploads
}
