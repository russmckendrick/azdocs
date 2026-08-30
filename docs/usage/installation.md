# Installation

## Release binaries

Download from the GitHub releases page — fully static binaries for:

- macOS (Apple Silicon and Intel)
- Linux (musl, x86_64 and arm64)
- Windows (x86_64)

No OpenSSL, no system SQLite, no Azure CLI required.

These are the **CLI** only. The desktop explorer is not yet published as a
release artifact — build it from source as below.

## From source

Requires a Rust toolchain:

```sh
cargo install --path .
```

### Desktop explorer

Additionally requires Node 22 and pnpm, plus the
[Tauri system dependencies](https://tauri.app/start/prerequisites/) for your
platform:

```sh
cd desktop
pnpm install
pnpm run tauri build     # bundles into target/release/bundle/
pnpm run tauri dev       # or run it directly
```

## Shell completions

```sh
azdocs completions zsh > ~/.zfunc/_azdocs
azdocs completions bash > /etc/bash_completion.d/azdocs
azdocs completions fish > ~/.config/fish/completions/azdocs.fish
```

Next: [Configuration](configuration.md)
