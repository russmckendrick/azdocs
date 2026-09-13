# Troubleshooting

## Common errors

**`AADSTS7000215: Invalid client secret`** — wrong or expired secret. Service
principal secrets expire. Create a replacement through your normal Entra
credential-rotation process, update the named profile with
`azdocs --tenant <reference> config set-secret` or update its environment secret,
then test the connection. Retire the old credential after all consumers have
switched. A credential reset can affect other clients using the same app.

**`azdocs check` shows fewer subscriptions than expected** — the service
principal may lack access to those subscriptions, the wrong tenant may be
selected, or the configured subscription list may exclude them. Check the
selected identity, scope and permission diagnostic details.

**`ARG throttled (429); backing off` during collect** — normal on larger
tenants; azdocs paces from quota headers and retries automatically. Lower
`--concurrency` if it persists. See [Collecting](collecting.md#throttling).

**`resource graph returned HTTP 400: BadRequest`** on a custom query — the
KQL is invalid for ARG (the classic: naming a column `count`). Test with
`azdocs query run ./file.toml` before adding it to the pack.

**Reports look stale** — reports come from a stored snapshot, not live Azure.
Run `azdocs collect` first.

## Limitations

The pack collects accessible control-plane configuration and service evidence:

- RBAC assignments and role definitions are available through ARG, but the
  stored inventory is not a resolved effective-access graph or an Entra object
  directory. Live ARM permission diagnostics have their own
  [coverage limits](permissions.md#evidence-and-limits).
- Blob contents, Key Vault secret values, database contents and data-plane
  access checks are outside the collection scope.
- Advisor recommendations contain estimated savings, not billed costs or
  measured utilisation. Resource changes are a retained subset of control-plane
  events, not a complete activity log.
- Some resource properties are partial or delayed compared with a direct ARM
  `GET`. Service configuration, permissions and retention can leave evidence
  empty or missing. A successful empty query is not a passed control.
- Desktop website images show what an isolated webview could render at capture
  time. A login or error page is not proof of application health.

See [Operational evidence](../reference/operational-evidence.md) and
[Website screenshots](website-screenshots.md) for source-specific limits.

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
