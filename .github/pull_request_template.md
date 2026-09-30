## Summary

<!-- What does this change and why? Link the issue it addresses, e.g. "Closes #123". -->

## How was it tested?

<!-- Tests added or updated, and anything checked by hand. -->

## Checklist

- [ ] Tests cover the change and pass locally
- [ ] `cargo fmt`, clippy (`-D warnings`) and/or `pnpm typecheck && pnpm lint` are clean
- [ ] New or changed `sqlx::query!` macros: `backend/.sqlx` refreshed with `cargo sqlx prepare --workspace`
- [ ] Documentation updated (README configuration reference, frontend README, chart values) when behavior or settings change
- [ ] Commit messages follow Conventional Commits
