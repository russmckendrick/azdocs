# Releasing

## CI

`.github/workflows/ci.yml` runs two jobs on every push and PR.

**`test`**, across ubuntu-latest, macos-latest and windows-latest:

- `cargo fmt --all --check` (covers both workspace members)
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --locked`

**`desktop`**, on ubuntu-latest, after installing the Tauri system libraries:

- `pnpm install --frozen-lockfile`, then `pnpm run lint`, `pnpm run typecheck`,
  `pnpm test` and `pnpm run build`
- `cargo clippy -p azdocs-desktop --all-targets --locked -- -D warnings`
- `cargo test -p azdocs-desktop --locked`

The frontend uses pnpm, not npm. Both jobs share one lockfile and one `target/`
now that `desktop/src-tauri` is a workspace member.

> Not yet covered: `pnpm run tauri build` is never exercised, so the packaged
> bundle is only compiled at release time, and the desktop job is Linux-only.

## Release builds

Tagging `v*` triggers `.github/workflows/release.yml`:

```mermaid
flowchart LR
    tag[git tag v0.2.0] --> build[Build matrix]
    build --> mac1[macOS aarch64]
    build --> mac2[macOS x86_64]
    build --> lin1[Linux musl x86_64]
    build --> lin2[Linux musl aarch64]
    build --> win[Windows x86_64]
    mac1 & mac2 & lin1 & lin2 & win --> rel[GitHub release<br/>with archives + notes]
```

```sh
git tag v0.2.0
git push origin v0.2.0
```

The musl builds are fully static; every artifact is a single dependency-free
binary.
