# Configuration

## Create a read-only service principal

azdocs authenticates as a service principal with the **Reader** role — it can
never modify anything.

```sh
az ad sp create-for-rbac --name azdocs-reader --role Reader \
    --scopes /subscriptions/<subscription-id>
```

Grant Reader on every subscription you want visible, or once on a management
group to cover them all. The command returns `tenant`, `appId`, and
`password` — the three values azdocs needs.

## Write the config

```sh
azdocs init    # interactive; chmod 600 on unix
azdocs check   # verifies token + lists visible subscriptions
```

`init` writes `azdocs.toml` to the platform config directory:

| OS | Config file | Database |
|---|---|---|
| macOS | `~/Library/Application Support/azdocs/azdocs.toml` | `~/Library/Application Support/azdocs/azdocs.db` |
| Linux | `~/.config/azdocs/azdocs.toml` | `~/.local/share/azdocs/azdocs.db` |
| Windows | `%APPDATA%\azdocs\azdocs.toml` | `%APPDATA%\azdocs\azdocs.db` |

Search order: `--config <path>` → `./azdocs.toml` → platform config dir.
`--db <path>` overrides the database location for any command.

## The config file

```toml
[auth]
tenant_id = "..."
client_id = "..."
client_secret = "..."   # optional in the file — see below

[collect]
subscriptions = []       # empty = all visible to the credential
concurrency = 4          # parallel ARG queries

[audit]
required_tags = ["environment", "owner"]   # drives missing-tag findings

[storage]
db_path = "/path/to/azdocs.db"
```

## Environment overrides

Recommended for the secret on shared machines and CI — env vars beat file
values:

```sh
export AZDOCS_TENANT_ID=...
export AZDOCS_CLIENT_ID=...
export AZDOCS_CLIENT_SECRET=...
```

Next: [Collecting](collecting.md)
