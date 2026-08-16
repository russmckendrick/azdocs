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
