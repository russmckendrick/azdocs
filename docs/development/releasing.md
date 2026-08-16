# Releasing

## CI

`.github/workflows/ci.yml` runs on every push and PR:

- `cargo fmt --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --locked`

across ubuntu-latest, macos-latest, and windows-latest.

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
