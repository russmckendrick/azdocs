# Installation

## CLI from source

Use a current stable [Rust toolchain](https://rustup.rs/) and its platform C/C++
linker tools. Clone the repository and install the CLI:

```sh
git clone https://github.com/russmckendrick/azdocs.git
cd azdocs
cargo install --path . --locked
azdocs --version
```

Cargo installs into its binary directory (normally `~/.cargo/bin` on Unix or
`%USERPROFILE%\.cargo\bin` on Windows). Put that directory on `PATH` if the
last command is not found. Source installation needs network access to obtain
uncached dependencies; collecting Azure data needs credentials and connectivity.

The CLI bundles SQLite and uses rustls, so no system SQLite, OpenSSL or Azure
CLI installation is required to run it. macOS and Windows executables still
link operating-system libraries. Native secret storage uses macOS Keychain,
Windows Credential Manager or Linux Secret Service. A headless Linux host can
use [environment references](configuration.md#credentials-and-environment-variables)
without a keyring service.

## Release archives

The project is preparing its first release. Use source installation until
archives appear on the [GitHub Releases page](https://github.com/russmckendrick/azdocs/releases).
The tag-triggered workflow builds CLI archives for:

| Platform | Target |
|---|---|
| macOS Apple Silicon | `aarch64-apple-darwin` |
| macOS Intel | `x86_64-apple-darwin` |
| Linux x86-64 | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-unknown-linux-musl` |
| Windows x86-64 | `x86_64-pc-windows-msvc` |

Compare the downloaded archive with the matching entry in
`azdocs-checksums.sha256` before extracting it. Extract the matching archive and place `azdocs` (or `azdocs.exe`) on `PATH`.
Keep its licence and notice files with redistributed copies. These archives
contain the CLI; the workflow does not publish desktop installers.

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
`target/release/bundle/` at the repository root. A source build is not a signed
or notarised public desktop release. Continue with the
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
