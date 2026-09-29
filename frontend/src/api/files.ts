import { request } from './client'
import type { Node, Presigned, Upload, Version } from './types'

export const files = {
  createUpload: (sizeBytes: number, mimeType: string) =>
    request<Upload>('POST', '/uploads', { body: { size_bytes: sizeBytes, mime_type: mimeType } }),
  createFile: (storageKey: string, parentId: string | null, name: string) =>
    request<Node>('POST', '/files', {
      body: { storage_key: storageKey, parent_id: parentId, name },
    }),
  createVersion: (id: string, storageKey: string) =>
    request<Version>('POST', `/nodes/${id}/versions`, { body: { storage_key: storageKey } }),
  versions: (id: string) => request<Version[]>('GET', `/nodes/${id}/versions`),
  download: (id: string, versionId?: string) =>
    request<Presigned>('GET', `/nodes/${id}/download`, { query: { version_id: versionId } }),
}
