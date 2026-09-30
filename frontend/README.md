# Trovr web client

The browser UI for Trovr: file browser, direct-to-storage uploads and downloads, version history, sharing (people and public links), shared-with-me and trash views.

Built with Vite, React, TypeScript, Tailwind CSS and [shadcn/ui](https://ui.shadcn.com), TanStack Query and React Router.

## Development

Requirements: Node.js 22+ and pnpm 10+, plus a running Trovr API (by default on `http://localhost:8080`, see the backend).

```sh
pnpm install
pnpm dev        # http://localhost:5173, proxies /api to http://localhost:8080
```

| Script           | What it does                                                             |
| ---------------- | ------------------------------------------------------------------------ |
| `pnpm dev`       | Dev server with hot reload                                               |
| `pnpm build`     | Type-check and build to `dist/`                                          |
| `pnpm typecheck` | TypeScript only                                                          |
| `pnpm lint`      | oxlint                                                                   |
| `pnpm format`    | Prettier (Tailwind classes sorted)                                       |
| `pnpm test`      | Vitest + Testing Library; the API and object storage are mocked with MSW |

## How it talks to the backend

- **Same origin as the API.** The client calls relative `/api/v1/...` URLs, so the API needs no CORS. In development Vite proxies `/api`; in production serve `dist/` and route `/api` to the backend on the same host (e.g. two paths on one ingress). Every other path must fall back to `index.html` (client-side routes such as `/folders/:id`, `/s/:token` and `/auth/callback`).
- **Sign-in methods** come from `GET /api/v1/auth/config`: the login page shows the internal form, registration and/or "Sign in with SSO" accordingly.
- **File bytes never go through the API**: uploads `PUT` and downloads `GET` presigned URLs on the object storage directly.

## Object storage CORS

Because the browser talks to the bucket directly, the bucket must accept cross-origin requests from the web origin. Browsers preflight the upload `PUT` (it carries a signed `Content-Type`). With any S3-compatible backend that supports `PutBucketCors` (AWS S3, MinIO, Garage, …):

```json
{
  "CORSRules": [
    {
      "AllowedOrigins": ["https://drive.example.com"],
      "AllowedMethods": ["GET", "PUT"],
      "AllowedHeaders": ["content-type"],
      "MaxAgeSeconds": 3600
    }
  ]
}
```

```sh
aws s3api put-bucket-cors --bucket trovr --cors-configuration file://cors.json \
  --endpoint-url https://s3.example.com
```

For local development, allow `http://localhost:5173`.

## OIDC (Keycloak or any OIDC provider)

When the server runs with `APP__AUTH_MODE=oidc` or `both`, the client signs in with the Authorization Code flow and PKCE, using the issuer and client id the server exposes. Configure the client in your identity provider as:

- **Public client** (no secret), standard flow enabled, PKCE method `S256`.
- **Redirect URI**: `https://<web origin>/auth/callback`; **post-logout redirect URI**: `https://<web origin>/login`; **web origin**: `https://<web origin>`.
- **Audience**: the API only accepts access tokens whose `aud` contains `APP__OIDC_CLIENT_ID`. In Keycloak, add an _Audience_ mapper to the client (or a dedicated client scope) that includes the client itself in the access token audience.
- Scopes: `openid profile email`. Tokens are renewed silently when the provider issues refresh tokens.

## Sessions

Internal sign-ins get a one-hour access token. It is kept in memory and mirrored to `sessionStorage`, so it survives a reload but not closing the tab; when it expires (or the API answers 401) the user is sent back to sign in and returns to the page they were on.
