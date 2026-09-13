# Development

azdocs is a Cargo workspace containing the CLI/shared library and Tauri backend.
Install the [source-build prerequisites](../usage/installation.md) first.
Run these commands from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

cd desktop
pnpm install --frozen-lockfile
pnpm run lint
pnpm run typecheck
pnpm test
pnpm run build
```

For CLI-only work, `cargo test --locked` and
`cargo clippy --all-targets --locked -- -D warnings` target the root package.
Workspace commands also require the Tauri system libraries. Tests use synthetic
fixtures and mocked HTTP; they do not need an Azure account. Native credential
and screenshot smoke tests are separate; see [Testing](testing.md).

For intentional golden changes, install `cargo-insta` with
`cargo install cargo-insta --locked`, then use `cargo insta review`.
Generated contracts and documentation checks are described in [Testing](testing.md).

| Page | Covers |
|---|---|
| [Architecture](architecture.md) | Modules, data flow, design decisions and offline boundaries |
| [Desktop map](desktop-relationships.md) | Topology ownership, scope layouts, connectors, labels and cameras |
| [Website screenshots](website-screenshots.md) | Endpoint discovery, isolated native capture, saved evidence and offline exports |
| [Data model](data-model.md) | SQLite schema, migrations and snapshot diffing |
| [Azure display metadata](azure-metadata.md) | Friendly Azure values and the Microsoft refresh flow |
| [Testing](testing.md) | Test layers, fixture estate, goldens and documentation checks |
| [Contributing](contributing.md) | Recipes, style and crate gotchas |
| [Releasing](releasing.md) | Version gates, signed bundles, GitHub Releases and Homebrew publishing |

Project policies: [Contributing](../../CONTRIBUTING.md),
[Security](../../SECURITY.md), [MIT licence](../../LICENSE) and
[Third-party notices](../../THIRD_PARTY_NOTICES.md).
