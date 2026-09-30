import { ApiError } from '@/api/client'
import { strings } from '@/strings'

const messages: Record<string, string> = strings.errors

/** A user-facing message for any error, preferring the API's error code. */
export function errorMessage(error: unknown): string {
  if (error instanceof ApiError) return messages[error.code] ?? error.message
  return strings.errors.unexpected
}
