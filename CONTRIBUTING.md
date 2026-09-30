# Contributing to Trovr

Thanks for your interest in Trovr! Bug reports, ideas and pull requests are welcome. By participating you agree to follow the [code of conduct](CODE_OF_CONDUCT.md).

## Before you start

- **Bugs**: open an issue with the bug report template (what you did, what you expected, what happened, versions).
- **Features**: open an issue first to discuss the idea, so the design can be agreed on before you spend time on code.
- **Security issues**: do not open a public issue; report them privately through GitHub's *Report a vulnerability* button on the repository's Security tab.

## Development setup

Requirements:

- Rust (stable, edition 2024) — `rustup` recommended
- Node.js 22+ and pnpm 10+
- Docker — the backend integration tests start PostgreSQL and Garage containers with testcontainers, and the local stack uses docker-compose

### Run everything with docker-compose

```sh
./deploy/compose/init-env.sh
docker compose up -d --build --wait     # http://localhost:8080
./deploy/compose/smoke.sh               # end-to-end check
```

### Work on the backend

The workspace lives in `backend/`. Commands can run from the repository root with `--manifest-path`:

```sh
cargo fmt --all --manifest-path backend/Cargo.toml
cargo clippy --manifest-path backend/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path backend/Cargo.toml --workspace      # needs Docker
```

To run the API against the compose services, start only them (`docker compose up -d postgres garage garage-init`) and run `cargo run --manifest-path backend/Cargo.toml -p trovr` with the `APP__*` variables from the [configuration reference](README.md#configuration) (the compose file shows working values).

### Work on the frontend

```sh
cd frontend
pnpm install
pnpm dev            # http://localhost:5173, proxies /api to http://localhost:8080
pnpm typecheck && pnpm lint && pnpm test && pnpm build
```

The API and object storage are mocked with MSW in the frontend tests, so they do not need a backend. See [frontend/README.md](frontend/README.md).

## Database migrations and the sqlx cache

- Migrations are plain SQL files in `backend/migrations/`, applied in order at startup (they are embedded in the binary). Add one with `sqlx migrate add -r <name> --source backend/migrations` (from [`sqlx-cli`](https://crates.io/crates/sqlx-cli)); never edit a migration that has been released.
- Queries checked at compile time (`sqlx::query!` macros) are cached in `backend/.sqlx/` so the project builds without a database (`SQLX_OFFLINE=true`, as in the Docker build and CI). After adding or changing such a query, refresh the cache against a migrated database and commit it:

  ```sh
  cd backend && DATABASE_URL=postgres://… cargo sqlx prepare --workspace
  ```

## Conventions

- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `build:`, `ci:`, `chore:` — a single subject line in English.
- **Code**: `cargo fmt` and clippy with `-D warnings` for Rust; Prettier and oxlint for the frontend. Match the style of the surrounding code; comments explain *why*, not *what*.
- **Tests**: every behavior change comes with a test — integration tests for API routes (`backend/crates/api/tests/`), Vitest + Testing Library for the web client. Write the test first and see it fail.
- **Scope**: keep pull requests focused on one change; split large work into reviewable steps.

## Pull requests

1. Fork the repository and create a branch from `main`.
2. Make your change with tests, and run the checks above.
3. Open a pull request using the template: what changed, why, and how you tested it.
4. CI must pass (formatting, lints, tests, builds, Helm chart validation). A maintainer will review; please address feedback in new commits.

By contributing you agree that your contributions are licensed under the [Apache-2.0 license](LICENSE).
