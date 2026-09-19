# Data on disk and uninstalling

azdocs keeps everything it knows about an estate in local files. Nothing is
sent anywhere except the Azure requests a collect makes and the website
requests a screenshot capture makes.

## What azdocs writes

| Data | Where | Notes |
|---|---|---|
| Configuration `azdocs.toml` | macOS `~/Library/Application Support/azdocs/`, Linux `~/.config/azdocs/`, Windows `%APPDATA%\azdocs\config\` | Named tenants, defaults, branding; secrets by reference only |
| Snapshot database `azdocs.db` (+ `-wal`, `-shm`) | macOS `~/Library/Application Support/azdocs/`, Linux `~/.local/share/azdocs/`, Windows `%APPDATA%\azdocs\data\` | Every snapshot: resources, findings, relationships, query rows, exact executed queries, website screenshots as PNG blobs |
| Migration backups `azdocs.toml.backup-<id>` | beside the configuration | Written when a legacy file is migrated; may contain a plaintext secret from that legacy file |
| Desktop preferences `desktop-preferences.json` | beside the configuration | The chosen configuration path and tenant |
| Client secrets | the OS credential store (macOS Keychain, Windows Credential Manager, Linux Secret Service), entries named `azdocs/<tenant reference>` | Only for profiles with `secret_ref`; `secret_env` profiles store nothing |
| Exports | `./output/` for the CLI, the folder you pick in the desktop | Reports, diagrams, CSV and XLSX; regenerate them from the database at any time |
| Desktop window and view state | the browser storage of the app's webview, and the OS window-state file Tauri keeps | Theme, sidebar, text scale, explorer filters, window size |

Override the database and configuration locations with `--db` and `--config`
or `[storage] db_path`; see [configuration](configuration.md).

The database and its `-wal`/`-shm` companions are one unit: copy or delete
them together, and never delete the companions while a collect is running.
`azdocs snapshots verify` checks the file, and `snapshots delete --vacuum`
or `prune --vacuum` returns space after removing snapshots.

## Sensitivity

The database holds resource configuration, identifiers, findings, executed
queries and any captured website images, which can include login pages.
Reports and exports carry the same evidence. Protect these files with the
same controls as the estate they describe; azdocs does not encrypt them.
See [SECURITY.md](../../SECURITY.md).

## Uninstall

1. Remove the program:
   - Homebrew: `brew uninstall azdocs` and `brew uninstall --cask azdocs-desktop`
   - Direct CLI download: delete the extracted directory or `~/.local/bin/azdocs`
   - macOS DMG: drag `azdocs.app` to the Bin
   - Windows: *Settings › Apps* (NSIS or MSI both register an uninstaller)
   - Linux: `apt remove azdocs-desktop`, `dnf remove azdocs-desktop`, or delete the AppImage
2. Remove the data if you no longer need the history: the configuration
   directory, the database with its `-wal`/`-shm` files, the backups and
   `desktop-preferences.json` from the table above, and any `output/` folders.
3. Remove stored secrets from the credential store: search for entries
   beginning `azdocs/`. Keychain Access on macOS, Credential Manager on
   Windows, Seahorse or `secret-tool` on Linux.

Uninstalling the program never deletes the database; step 2 is a choice.

Next: [Troubleshooting](troubleshooting.md)
