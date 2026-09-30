import type { Presigned } from '@/api/types'

/**
 * Lets the browser fetch a presigned GET: the URL already asks storage for an
 * `attachment` disposition, so the file is saved and the bytes skip the API.
 */
export function startDownload(presigned: Presigned) {
  const link = document.createElement('a')
  link.href = presigned.url
  link.rel = 'noopener'
  document.body.appendChild(link)
  link.click()
  link.remove()
}
