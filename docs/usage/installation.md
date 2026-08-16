# Installation

## Release binaries

Download from the GitHub releases page — fully static binaries for:

- macOS (Apple Silicon and Intel)
- Linux (musl, x86_64 and arm64)
- Windows (x86_64)

No OpenSSL, no system SQLite, no Azure CLI required.

## From source

Requires a Rust toolchain:

```sh
cargo install --path .
```

## Shell completions

```sh
azdocs completions zsh > ~/.zfunc/_azdocs
azdocs completions bash > /etc/bash_completion.d/azdocs
azdocs completions fish > ~/.config/fish/completions/azdocs.fish
```

Next: [Configuration](configuration.md)
