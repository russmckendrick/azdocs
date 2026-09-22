# Command-line reference

Every `azdocs` subcommand shares the global flags below. Subcommand-specific
options are covered on their own pages; this page collects the flags and
environment variables that are easy to miss.

## Global flags

| Flag | Effect |
|---|---|
| `--config <path>` | Use this config file instead of `./azdocs.toml` or the platform config file. A path that does not exist is an error, not a fallback to defaults. |
| `--db <path>` | Use this SQLite database for the invocation. |
| `--tenant <reference-or-uuid>` | Select a named profile or a tenant id when the selection is ambiguous. |
| `-v` / `-vv` | Log at `info` / `debug` on stderr. The default is `warn`. |
| `--no-color` | Disable ANSI colour in log output. |

## Environment variables

| Variable | Effect |
|---|---|
| `AZDOCS_TENANT_ID`, `AZDOCS_CLIENT_ID`, `AZDOCS_CLIENT_SECRET` | Legacy environment-only identity. Named profiles reference their secret through `secret_env` instead. |
| `NO_COLOR` | Any non-empty value disables colour, the same as `--no-color`. |
| `RUST_LOG` | A tracing filter that replaces the `-v` level entirely, for example `RUST_LOG=azdocs::arg=debug`. |
| `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` | Honoured for every Azure request. Operating-system proxy settings are not read; set the variables explicitly on corporate networks. |

## Machine-readable output

`check`, `snapshots list`, `snapshots show`, `snapshots verify` and
`query list` accept `--format json` and print one JSON document on stdout.
Human tables stay the default. `snapshots diff` and `query run` have their own
`--format` sets (`table|md|json` and `table|json|csv`).

```sh
azdocs check --format json | jq .ok
azdocs snapshots show latest --format json | jq '.query_runs[] | select(.error != null)'
```

## Exit codes

- `0` on success.
- `1` on any error, including a `check` whose credential sees no subscriptions
  or cannot describe its own identity, and a `collect` whose outcome matches
  `--fail-on`.
- `2` when the command line itself is invalid (clap's usage error).

`collect --fail-on failed` is the default: the command exits non-zero only when
every query failed or collection was cancelled. `--fail-on partial` also fails
for audit warnings and incomplete inventory, including dropped inventory rows.
The snapshot line is always printed first, so a pipeline still learns the id.

## `init`

```sh
azdocs init                                   # interactive
azdocs init --force                           # overwrite an existing file
azdocs init --secret-env AZDOCS_ACME_SECRET   # never store the secret; reference a variable
AZDOCS_TENANT_ID=… AZDOCS_CLIENT_ID=… azdocs init --non-interactive
```

Non-interactive runs take identity from the environment and always write a
`secret_env` reference (`--secret-env`, else `AZDOCS_CLIENT_SECRET`); nothing is
written to an OS credential store, so headless hosts need none. Leaving the
interactive secret prompt empty does the same.

## `collect`

```sh
azdocs collect --dry-run          # list the queries and subscriptions; contacts nothing
azdocs collect --quiet            # only the final snapshot line
azdocs collect --fail-on partial  # exit non-zero when any query failed
```

`--subscriptions` must be subscription UUIDs and `--concurrency` must be
between 1 and 64; the command line applies the same rules as the config file.

## `snapshots`

```sh
azdocs snapshots list --format json
azdocs snapshots show <id> --format json
azdocs snapshots delete <id> --yes [--force] [--vacuum]
azdocs snapshots prune --keep 5 --yes [--vacuum]
azdocs snapshots verify [--format json]
```

See [Snapshots](snapshots.md) for what each does.

## `query`

```sh
azdocs query list --category security --format json
azdocs query run nsg_open_to_internet --subscriptions <uuid>
azdocs query run - < ./adhoc.kql
```

## `browse`

```sh
azdocs browse --snapshot <id>     # default: latest
```

Next: [Configuration](configuration.md)
