# Troubleshooting

## Common errors

**`AADSTS7000215: Invalid client secret`** — wrong or expired secret. Service
principal secrets expire:

```sh
az ad sp credential reset --id <appId>
```

**`azdocs check` shows fewer subscriptions than expected** — the service
principal lacks the Reader role on the missing subscriptions.

**`ARG throttled (429); backing off` during collect** — normal on larger
tenants; azdocs paces from quota headers and retries automatically. Lower
`--concurrency` if it persists. See [Collecting](collecting.md#throttling).

**`resource graph returned HTTP 400: BadRequest`** on a custom query — the
KQL is invalid for ARG (the classic: naming a column `count`). Test with
`azdocs query run ./file.toml` before adding it to the pack.

**Reports look stale** — reports come from a stored snapshot, not live Azure.
Run `azdocs collect` first.

## Limitations

Azure Resource Graph exposes control-plane configuration only:

- No RBAC role assignments, no Entra ID objects
- No data-plane state (blob contents, secrets, SQL logins)
- No cost/billing data, no activity logs
- Some types' `properties` are partial versus a direct ARM `GET` (ARG serves
  a cached projection)

The audit is therefore a **configuration** audit — and read-only by
construction.

## Configuration and Settings

- **Multiple tenants, no selection:** choose a tenant in the toolbar or pass
  `--tenant <reference-or-id>`. Set a default in Settings for CLI commands.
- **Configuration changed outside the app:** discard the stale draft, reload
  from disk and reapply edits. Saves intentionally refuse revision conflicts.
- **Credential store unavailable/locked:** unlock Keychain, Credential Manager
  or Secret Service. On a headless machine, use a profile's explicit `secret_env`.
  No plaintext fallback is attempted.
- **Unable to verify permissions:** open the diagnostic details for missing
  identity, inaccessible scopes, unreadable definitions or unsupported patterns.
  The result does not mean read-only access was proved. See [permission checks](permissions.md).
- **Invalid TOML:** Settings remains accessible and provides the file path and a
  sanitised error. Repair the file or load another configuration, then reload.
- **History disappears after changing tenant:** snapshots are filtered by tenant.
  Select the former tenant (including unconfigured tenant IDs) to browse them.
- **A tested secret expired before saving:** draft secret tokens expire after
  15 minutes. Re-enter and test or save the secret again.
- **Settings cannot scroll or Save is off-screen:** current Settings has a
  bounded scroll area and a separate action bar. Reload/rebuild the desktop
  frontend if an older development bundle remains open.
