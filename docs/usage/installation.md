# Installation

## Homebrew

Install the CLI on macOS or Linux:

```sh
brew install russmckendrick/tap/azdocs
```

On Apple Silicon macOS, install the signed and notarized desktop app:

```sh
brew install --cask russmckendrick/tap/azdocs-desktop
```

The desktop cask is currently Apple Silicon only. Windows and Linux desktop
packages are available from
[GitHub Releases](https://github.com/russmckendrick/azdocs/releases).

## Direct downloads

Each release publishes these CLI archives:

| Platform | Asset |
|---|---|
| macOS Apple Silicon | `azdocs-darwin-arm64.tar.gz` |
| macOS Intel | `azdocs-darwin-amd64.tar.gz` |
| Linux x86-64 | `azdocs-linux-amd64.tar.gz` |
| Linux ARM64 | `azdocs-linux-arm64.tar.gz` |
| Windows x86-64 | `azdocs-windows-amd64.zip` |

Desktop packages are published for:

| Platform | Assets |
|---|---|
| macOS Apple Silicon | Signed and notarized DMG |
| Windows x86-64 | NSIS setup executable and MSI |
| Linux x86-64 and ARM64 | AppImage, DEB and RPM |

Compare direct downloads with `azdocs-checksums.sha256`. Extract a CLI archive
and place `azdocs` (or `azdocs.exe`) on `PATH`. Keep the included licence
and notice files with redistributed copies.

The CLI bundles SQLite and uses rustls, so no system SQLite, OpenSSL or Azure
CLI installation is required. Native secret storage uses macOS Keychain,
Windows Credential Manager or Linux Secret Service. A headless Linux host can
use [environment references](configuration.md#credentials-and-environment-variables)
without a keyring service.

## CLI from source

Use a current stable [Rust toolchain](https://rustup.rs/) and its platform C/C++
linker tools:

```sh
git clone https://github.com/russmckendrick/azdocs.git
cd azdocs
cargo install --path . --locked
azdocs --version
```

Cargo installs into its binary directory, normally `~/.cargo/bin` on Unix or
`%USERPROFILE%\.cargo\bin` on Windows. Put that directory on `PATH` if the
last command is not found.

## Desktop from source

The desktop additionally needs Node.js 22.12+ (22.x), pnpm 10 and the
[Tauri v2 system prerequisites](https://v2.tauri.app/start/prerequisites/) for
your platform. Linux needs the WebKitGTK development libraries; Windows uses
WebView2 and Microsoft C++ build tools; macOS needs Xcode command-line tools.

From the repository root:

```sh
cd desktop
pnpm install --frozen-lockfile
pnpm run tauri dev
```

To create a platform application bundle or installer instead:

```sh
pnpm run tauri build
```

Cargo output is in the workspace's `target/`; bundles are under
`target/release/bundle/` at the repository root. A local source build is not
the signed and notarized macOS artifact published by the release workflow.
Continue with the
[desktop first-run settings](desktop.md#settings).

## Shell completions

For zsh, create a completion directory and generate the definition:

```sh
mkdir -p ~/.zfunc
azdocs completions zsh > ~/.zfunc/_azdocs
```

Add `~/.zfunc` to `fpath` before `compinit` in your zsh configuration. For fish:

```sh
mkdir -p ~/.config/fish/completions
azdocs completions fish > ~/.config/fish/completions/azdocs.fish
```

For an existing Bash completion setup, source the generated file from your shell
configuration:

```sh
mkdir -p ~/.local/share/bash-completion/completions
azdocs completions bash > ~/.local/share/bash-completion/completions/azdocs
```

Next: [Configuration](configuration.md)
