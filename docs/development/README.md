# Development

```sh
cargo test                                        # full suite — no Azure/network needed
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
cargo insta review                                # accept intended golden-output changes
```

| Page | Covers |
|---|---|
| [Architecture](architecture.md) | Modules, data flow, the design decisions and why |
| [Data model](data-model.md) | SQLite schema, migrations, snapshot diffing |
| [Testing](testing.md) | Test layers, the fixture estate, golden files |
| [Contributing](contributing.md) | Recipes for common changes, style, crate gotchas |
| [Releasing](releasing.md) | CI matrix and the tag-triggered release build |
