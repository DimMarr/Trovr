/** A failed API call: `code` is the API's machine-readable error code. */
export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

interface ApiConfig {
  /** The current access token, if signed in. */
  getToken: () => string | null
  /** Called when an authenticated call is refused with 401. */
  onUnauthorized: () => void
}

let config: ApiConfig = { getToken: () => null, onUnauthorized: () => {} }

export function configureApi(next: ApiConfig) {
  config = next
}

export type Method = 'GET' | 'POST' | 'DELETE'

interface RequestOptions {
  body?: unknown
  query?: Record<string, string | undefined>
  /** Send the bearer token and report 401s (default: true). */
  auth?: boolean
}

export async function request<T>(
  method: Method,
  path: string,
  { body, query, auth = true }: RequestOptions = {},
): Promise<T> {
  const url = new URL(`/api/v1${path}`, window.location.origin)
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value !== undefined) url.searchParams.set(key, value)
  }

  const headers = new Headers()
  const token = auth ? config.getToken() : null
  if (token) headers.set('Authorization', `Bearer ${token}`)
  if (body !== undefined) headers.set('Content-Type', 'application/json')

  let response: Response
  try {
    response = await fetch(url, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    })
  } catch {
    throw new ApiError(0, 'network_error', 'The server could not be reached.')
  }

  if (!response.ok) {
    if (response.status === 401 && auth) config.onUnauthorized()
    throw await toApiError(response)
  }
  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

async function toApiError(response: Response): Promise<ApiError> {
  try {
    const { error } = (await response.json()) as { error: { code: string; message: string } }
    return new ApiError(response.status, error.code, error.message)
  } catch {
    return new ApiError(response.status, `http_${response.status}`, response.statusText)
  }
}
