# Trovr

**A self-hostable, cloud-native file drive.** Store and organize files in folders, keep every version, share with people or through public links — on your own PostgreSQL and any S3-compatible object storage.

- **Stateless and horizontally scalable**: one Rust binary, all state in PostgreSQL (metadata) and object storage (bytes). Any replica can serve any request.
- **Direct-to-storage transfers**: browsers upload and download through presigned URLs; file bytes never go through the API.
- **Pluggable sign-in**: built-in accounts (argon2, RS256 JWTs), any OIDC-compliant provider such as Keycloak, or both.
- **Sharing**: viewer/editor access inherited by whole folders, revocable and optionally expiring read-only public links.
- **Versions and trash**: every upload of a file keeps the previous versions downloadable; deleted items go to a restorable trash.
- **S3-compatible storage**: AWS S3, Garage, MinIO, OVHcloud Object Storage, … (Garage is used for local development and the integration tests).

## Architecture

```mermaid
flowchart LR
    browser["Browser<br/>(React SPA)"]
    web["trovr-web<br/>nginx: SPA + /api proxy"]
    api["trovr-api<br/>Rust / axum"]
    db[("PostgreSQL<br/>metadata")]
    s3[("S3-compatible storage<br/>file bytes")]
    idp["OIDC provider<br/>(optional)"]

    browser -- "pages, /api (JSON)" --> web
    web -- "/api" --> api
    api --> db
    api -- "presigns URLs,<br/>checks and deletes objects" --> s3
    browser -- "presigned PUT / GET<br/>(file bytes)" --> s3
    browser -. "sign-in (PKCE)" .-> idp
    api -. "JWKS" .-> idp
```

The backend is a modular monolith, a Cargo workspace under [`backend/`](backend/):

| Crate | Responsibility |
|---|---|
| `config` | Configuration from `APP__*` environment variables |
| `auth` | `TokenValidator` with internal (argon2 + RS256) and OIDC (discovery + JWKS) implementations, user directory |
| `metadata` | Folders, files, versions, trash, sharing and effective roles over PostgreSQL (recursive CTEs) |
| `storage` | S3 client wrapper: presigned upload/download URLs, object checks and deletes |
| `api` | axum REST API tying the above together |
| `app` | The `trovr` binary: wiring, migrations, graceful shutdown |

The web client lives in [`frontend/`](frontend/) (React, TypeScript, Tailwind CSS, shadcn/ui) — see its [README](frontend/README.md).

## Quickstart (docker-compose)

Requirements: Docker with Compose v2, `openssl`; `curl` and `jq` for the smoke test.

```sh
git clone https://github.com/DimMarr/Trovr.git && cd Trovr
./deploy/compose/init-env.sh          # generates .env: JWT keys and dev credentials
docker compose up -d --build --wait
```

Open http://localhost:8080 and create an account. The stack runs PostgreSQL, [Garage](https://garagehq.deuxfleurs.fr/) as the object storage (its S3 port 3900 is published for the browser), the API and the web client. `./deploy/compose/smoke.sh` runs an end-to-end check (sign-up, upload to storage, download).

Stop with `docker compose down` (add `-v` to delete the data).

## Configuration

The API reads its configuration from environment variables.

| Variable | Default | Description |
|---|---|---|
| `APP__BIND_ADDR` | *required* (`0.0.0.0:8080` in the image) | Address the HTTP server listens on |
| `APP__DATABASE_URL` | *required* | PostgreSQL URL, e.g. `postgres://user:pass@host:5432/trovr`; migrations run at startup |
| `APP__LOG_FORMAT` | `pretty` (`json` in the image) | `pretty` or `json` |
| `RUST_LOG` | `info` | Log filter, e.g. `info,trovr_api=debug` |
| `APP__AUTH_MODE` | `internal` | `internal`, `oidc` or `both` |
| `APP__JWT_PRIVATE_KEY_PEM` | *required* | RSA private key (PEM) signing internal access tokens |
| `APP__JWT_PUBLIC_KEY_PEM` | *required* | Matching public key (PEM) |
| `APP__AUTH_ALLOW_REGISTRATION` | `false` | Whether anyone may create an internal account |
| `APP__OIDC_ISSUER_URL` | — | OIDC issuer; required with `oidc` or `both` |
| `APP__OIDC_CLIENT_ID` | — | OIDC client id; access tokens must carry it in `aud`; required with `oidc` or `both` |
| `APP__S3_ENDPOINT_URL` | — (AWS S3) | How the API reaches the S3-compatible storage |
| `APP__S3_PUBLIC_ENDPOINT_URL` | same as `APP__S3_ENDPOINT_URL` | How browsers reach it, when different (presigned URLs embed this host) |
| `APP__S3_REGION` | `us-east-1` | Region used for signing |
| `APP__S3_BUCKET` | *required* | Bucket holding file contents |
| `APP__S3_ACCESS_KEY_ID` | *required* | S3 access key |
| `APP__S3_SECRET_ACCESS_KEY` | *required* | S3 secret key |
| `APP__S3_FORCE_PATH_STYLE` | `true` | Path-style addressing (needed by most self-hosted backends) |
| `APP__MAX_UPLOAD_BYTES` | `5368709120` (5 GiB) | Largest accepted file (a single presigned PUT is capped at 5 GiB) |
| `APP__UPLOAD_URL_TTL_SECONDS` | `900` | Lifetime of presigned upload URLs |
| `APP__DOWNLOAD_URL_TTL_SECONDS` | `300` | Lifetime of presigned download URLs |

Generate a key pair for internal sign-in with:

```sh
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out jwt.pem
openssl pkey -in jwt.pem -pubout -out jwt.pub.pem
```

The bucket must allow CORS (`GET`, `PUT`) from the web origin, since browsers talk to it directly; for OIDC, the client must be public with PKCE. Both are detailed in the [frontend README](frontend/README.md).

## Deploying on Kubernetes

A Helm chart lives in [`deploy/helm/trovr`](deploy/helm/trovr). PostgreSQL and the object storage are external.

```sh
helm install trovr ./deploy/helm/trovr \
  --set api.config.APP__S3_ENDPOINT_URL=https://s3.example.com \
  --set api.config.APP__S3_BUCKET=trovr \
  --set secrets.existingSecret=trovr-secrets \
  --set ingress.enabled=true --set ingress.host=drive.example.com
```

`trovr-secrets` holds `APP__DATABASE_URL`, `APP__JWT_PRIVATE_KEY_PEM`, `APP__JWT_PUBLIC_KEY_PEM`, `APP__S3_ACCESS_KEY_ID` and `APP__S3_SECRET_ACCESS_KEY` (or set them under `secrets.*` and let the chart create it). Every `APP__*` setting above goes under `api.config`. The Ingress routes `/api` to the API and everything else to the web client on one host. Pods run as non-root with a read-only root filesystem. See [`values.yaml`](deploy/helm/trovr/values.yaml) for all options.

Images are published to `ghcr.io/dimmarr/trovr-api` and `ghcr.io/dimmarr/trovr-web` for every commit on `main` and every `v*` tag.

## Project layout

```
backend/            Rust workspace (crates/, migrations/, .sqlx/ offline query cache)
frontend/           React web client
deploy/compose/     docker-compose helpers (Garage config and init, env generator, smoke test)
deploy/helm/trovr/  Helm chart
docker-compose.yml  Local stack
```

## Contributing

Contributions are welcome: read [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).

## License

[Apache-2.0](LICENSE)
