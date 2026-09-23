# Installation

## Homebrew

The public tap contains both the CLI formula and the macOS desktop cask. Install
the CLI on macOS or Linux with either the fully qualified name:

```sh
brew install russmckendrick/tap/azdocs
```

or tap the repository once and use the shorter name thereafter:

```sh
brew tap russmckendrick/tap
brew install azdocs
azdocs --version
```

On Apple Silicon macOS, install the signed and notarized desktop app from the
same tap:

```sh
brew install --cask russmckendrick/tap/azdocs-desktop
```

The desktop cask is currently Apple Silicon only. Windows and Linux desktop
packages are available from
[GitHub Releases](https://github.com/russmckendrick/azdocs/releases).

Homebrew upgrades both editions in the usual way:

```sh
brew update
brew upgrade azdocs
brew upgrade --cask azdocs-desktop
```

## GitHub Releases

The [latest release](https://github.com/russmckendrick/azdocs/releases/latest)
can be downloaded in a browser or with the
[GitHub CLI](https://cli.github.com/). Every package has a SHA-256 checksum;
`azdocs-checksums.sha256` contains the complete release manifest.

### CLI

Each release publishes these CLI archives:

| Platform | Asset |
|---|---|
| macOS Apple Silicon | `azdocs-darwin-arm64.tar.gz` |
| macOS Intel | `azdocs-darwin-amd64.tar.gz` |
| Linux x86-64 | `azdocs-linux-amd64.tar.gz` |
| Linux ARM64 | `azdocs-linux-arm64.tar.gz` |
| Windows x86-64 | `azdocs-windows-amd64.zip` |

On macOS or Linux, choose the asset from the table and replace the name in
these commands if necessary. This Apple Silicon example installs into the
per-user `~/.local/bin` directory:

```sh
mkdir -p azdocs-download ~/.local/bin
cd azdocs-download
gh release download --repo russmckendrick/azdocs \
  --pattern 'azdocs-darwin-arm64.tar.gz*'
shasum -a 256 -c azdocs-darwin-arm64.tar.gz.sha256
tar -xzf azdocs-darwin-arm64.tar.gz
install -m 0755 azdocs ~/.local/bin/azdocs
~/.local/bin/azdocs --version
```

Linux can use `sha256sum -c` in place of `shasum -a 256 -c`. Ensure
`~/.local/bin` is on `PATH` if it is not already.

On Windows, use PowerShell to download, verify and extract the x86-64 archive:

```powershell
New-Item -ItemType Directory -Force azdocs-download | Out-Null
Set-Location azdocs-download
gh release download --repo russmckendrick/azdocs `
  --pattern "azdocs-windows-amd64.zip*"
$expected = (Get-Content .\azdocs-windows-amd64.zip.sha256).Split()[0]
$actual = (Get-FileHash .\azdocs-windows-amd64.zip -Algorithm SHA256).Hash.ToLower()
if ($actual -ne $expected) { throw "azdocs checksum mismatch" }
Expand-Archive .\azdocs-windows-amd64.zip -DestinationPath .\azdocs
.\azdocs\azdocs.exe --version
```

Move the extracted directory to a permanent location and add it to the user
`PATH` if `azdocs.exe` should be available in every terminal.

### Desktop

Desktop packages are published for:

| Platform | Assets |
|---|---|
| macOS Apple Silicon | Signed and notarized DMG |
| Windows x86-64 | NSIS setup executable and MSI |
| Linux x86-64 and ARM64 | AppImage, DEB and RPM |

Download and verify the Apple Silicon DMG before opening it:

```sh
gh release download --repo russmckendrick/azdocs \
  --pattern 'azdocs-desktop-macos-arm64.dmg*'
shasum -a 256 -c azdocs-desktop-macos-arm64.dmg.sha256
open azdocs-desktop-macos-arm64.dmg
```

#### Windows SmartScreen

Windows installers, the desktop app and the CLI's `azdocs.exe` are signed
with a Certum Open Source code-signing certificate issued to
**Open Source Developer Russell McKendrick** when the release was built with
signing credentials; a release note says which. SmartScreen reputation builds
per file as a release is downloaded, so even a signed installer can show
**Windows protected your PC** for a few days after a release, and an
unsigned one always does. Verify the download's checksum (or its attestation with
`gh attestation verify`), then choose **More info › Run anyway**. The
installed app itself never contacts anything but Azure and the websites you
ask it to capture.

On Windows, download either the setup executable or MSI together with its
matching `.sha256` file, verify it with `Get-FileHash` as in the CLI example,
then run the installer.

On Linux, replace `amd64` with `arm64` when appropriate. Downloading the whole
set lets the shared checksum file verify the AppImage, DEB and RPM together:

```sh
gh release download --repo russmckendrick/azdocs \
  --pattern 'azdocs-desktop-linux-amd64.*'
sha256sum -c azdocs-desktop-linux-amd64.sha256
sudo apt install ./azdocs-desktop-linux-amd64.deb
```

Use the RPM with the distribution's package manager instead, or make the
AppImage executable with `chmod +x` and run it directly.

Keep the included licence and notice files with redistributed CLI copies.

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
