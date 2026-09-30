import { ApiError } from '@/api/client'
import { files } from '@/api/files'
import type { Presigned } from '@/api/types'

/** Headers the browser computes itself and refuses to let scripts set. */
const FORBIDDEN_HEADERS = new Set(['host', 'content-length', 'connection'])

export function mimeTypeOf(file: File): string {
  return file.type || 'application/octet-stream'
}

/** Sends `body` straight to object storage with the presigned request, reporting progress. */
export function putWithProgress(
  presigned: Presigned,
  body: Blob,
  onProgress?: (fraction: number) => void,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open(presigned.method, presigned.url)
    for (const [name, value] of Object.entries(presigned.headers)) {
      if (!FORBIDDEN_HEADERS.has(name.toLowerCase())) xhr.setRequestHeader(name, value)
    }
    xhr.upload.onprogress = (event) => {
      if (event.lengthComputable && event.total > 0) onProgress?.(event.loaded / event.total)
    }
    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) resolve()
      else
        reject(new ApiError(xhr.status, 'storage_upload_failed', `storage answered ${xhr.status}`))
    }
    xhr.onerror = () => reject(new ApiError(0, 'network_error', 'storage could not be reached'))
    xhr.send(body)
  })
}

/** Steps 1 and 2 of an upload: presign, then PUT the bytes. Returns the storage key to confirm. */
export async function uploadContent(
  file: File,
  onProgress?: (fraction: number) => void,
): Promise<string> {
  const { storage_key, upload } = await files.createUpload(file.size, mimeTypeOf(file))
  await putWithProgress(upload, file, onProgress)
  return storage_key
}
