# Development

```sh
cargo test                                        # full suite — no Azure/network needed
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
cargo insta review                                # accept intended golden-output changes

cd desktop
npm ci
npm run build
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

| Page | Covers |
|---|---|
| [Architecture](architecture.md) | Modules, data flow, the design decisions and why |
| [Data model](data-model.md) | SQLite schema, migrations, snapshot diffing |
| [Azure display metadata](azure-metadata.md) | Friendly resource types, locations, kinds, and the Microsoft refresh flow |
| [Testing](testing.md) | Test layers, the fixture estate, golden files |
| [Contributing](contributing.md) | Recipes for common changes, style, crate gotchas |
| [Releasing](releasing.md) | CI matrix and the tag-triggered release build |
