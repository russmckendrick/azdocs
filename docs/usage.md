# azdocs usage guide

azdocs audits an Azure estate with **read-only credentials**, stores everything
locally in SQLite as point-in-time **snapshots**, then exports reports and
diagrams entirely **offline**. Nothing is sent anywhere; the only network calls
are token acquisition and Azure Resource Graph queries during `collect`,
`check`, and `query run`.

- [Installation](#installation)
- [Setting up credentials](#setting-up-credentials)
- [Configuration](#configuration)
- [Collecting a snapshot](#collecting-a-snapshot)
- [Reports](#reports)
- [Diagrams](#diagrams)
- [Browsing with the TUI](#browsing-with-the-tui)
- [Working with snapshots](#working-with-snapshots)
- [Ad-hoc queries](#ad-hoc-queries)
- [Custom queries](#custom-queries)
- [Running in CI](#running-in-ci)
- [Troubleshooting](#troubleshooting)
- [Limitations](#limitations)

## Installation

From source (Rust toolchain required):

```sh
cargo install --path .
```

Or download a release binary (macOS arm64/x86_64, Linux musl x86_64/arm64,
Windows) from the GitHub releases page. The binary is fully static — no
OpenSSL, no system SQLite, no Azure CLI required.

Shell completions:

```sh
azdocs completions zsh > ~/.zfunc/_azdocs   # bash|zsh|fish|powershell|elvish
```

## Setting up credentials

azdocs authenticates as a **service principal with the Reader role** — it can
never modify anything. Create one:

```sh
az ad sp create-for-rbac --name azdocs-reader --role Reader \
    --scopes /subscriptions/<subscription-id>
```

Grant Reader on every subscription you want visible, or once on a management
group to cover them all. The command returns `tenant`, `appId`, and `password`
— those are the three values azdocs needs.

Then run the interactive setup and verify:

```sh
azdocs init    # writes the config file (chmod 600 on unix)
azdocs check   # token + probe query; lists visible subscriptions
```

## Configuration

`azdocs init` writes `azdocs.toml` to the platform config directory:

| OS | Config file | Database |
|---|---|---|
| macOS | `~/Library/Application Support/azdocs/azdocs.toml` | `~/Library/Application Support/azdocs/azdocs.db` |
| Linux | `~/.config/azdocs/azdocs.toml` | `~/.local/share/azdocs/azdocs.db` |
| Windows | `%APPDATA%\azdocs\azdocs.toml` | `%APPDATA%\azdocs\azdocs.db` |

Search order: `--config <path>` → `./azdocs.toml` → platform config dir.
`--db <path>` overrides the database location for any command.

```toml
[auth]
tenant_id = "..."
client_id = "..."
client_secret = "..."   # optional in the file — see env overrides

[collect]
subscriptions = []       # empty = all subscriptions visible to the credential
concurrency = 4          # parallel ARG queries

[audit]
required_tags = ["environment", "owner"]   # drives the missing-tags findings

[storage]
db_path = "/path/to/azdocs.db"
```

Environment variables override file values — recommended for the secret on
shared machines and CI:

- `AZDOCS_TENANT_ID`
- `AZDOCS_CLIENT_ID`
- `AZDOCS_CLIENT_SECRET`

## Collecting a snapshot

```sh
azdocs collect
```

Runs the full query pack (41 queries — see [queries.md](queries.md)) across
all configured subscriptions and stores the results as a new snapshot. Useful
flags:

```sh
azdocs collect --subscriptions <id>,<id>     # scope to specific subscriptions
azdocs collect --categories networking,security
azdocs collect --queries all_resources       # just named queries
azdocs collect --skip-queries orphaned_resources
azdocs collect --concurrency 2               # gentler on ARG quota
azdocs collect --notes "pre-migration baseline"
```

A snapshot ends in one of three states: `complete` (all queries succeeded),
`partial` (some failed — the rest of the data is still usable), or `failed`.
Failures per query are recorded; inspect them with `azdocs snapshots show`.

After the queries finish, two offline post-passes run automatically:

- **Edge extraction** — relationships (subnets, peerings, NSG associations,
  NIC/VM/disk attachment, private endpoints, DNS links) are derived from the
  stored properties JSON. No extra API calls.
- **Tag audit** — every resource is checked against `[audit] required_tags`;
  misses become `low` severity findings.

## Reports

```sh
azdocs report --format md|html|csv|xlsx|all   [--snapshot <id|latest>] [--out <dir>]
```

Everything lands under `./output/` by default:

| Format | Output | Contents |
|---|---|---|
| `md` | `output/docs/` | Markdown tree: index, findings, per-category pages, per-subscription pages, and per-resource-group **detail pages** (`resources/<sub>/<rg>.md`) with settings tables, finding callouts, and related-resource links |
| `html` | `output/report.html` + `output/docs-html/` | Single self-contained report (severity badges, client-side table filter) **and** a multi-page HTML twin of the docs tree |
| `csv` | `output/inventory.csv`, `output/findings.csv` | Flat exports for spreadsheets/scripts |
| `xlsx` | `output/azdocs.xlsx` | Workbook: Summary, Inventory (autofilter), Findings (severity colours), one sheet per category |

The markdown docs work as-is in any Git host or wiki. The HTML docs are plain
static files — open locally or drop on any web server.

## Diagrams

```sh
azdocs diagram --type hierarchy|resources|network \
    [--format drawio|mermaid|both] [--subscription <id>] [--resource-group <name>] [--out <path>]
```

| Type | Shows |
|---|---|
| `hierarchy` | Tenant → subscriptions → resource groups with resource counts |
| `resources` | Resource-group containers with an icon per resource and attachment edges |
| `network` | VNets and subnets as containers, VMs placed in their subnets, dashed peering edges, NSG associations, private-endpoint links, plus a "Connected services" group for NSGs and private-link targets |

Formats: `.drawio` files use the azure2 icon set and are fully editable in
[draw.io](https://app.diagrams.net) or the VS Code draw.io extension; `.mmd`
Mermaid files render directly in GitHub markdown and docs sites.

Large estates get unreadable in one diagram — over ~150 nodes azdocs warns and
suggests scoping with `--subscription` / `--resource-group`.

## Browsing with the TUI

```sh
azdocs browse [--snapshot <id|latest>]
```

Three screens:

- **Snapshot picker** — pick which snapshot to explore.
- **Estate view** — three panes: subscription/resource-group tree, filterable
  resource list, and a detail pane with tags, related edges, and the full
  pretty-printed properties JSON.
- **Findings** — severity-ordered list; `Enter` jumps straight to the affected
  resource's detail.

| Key | Action |
|---|---|
| `↑↓` / `jk` | Move |
| `Tab` | Cycle panes |
| `Enter` | Drill in / open |
| `/` | Incremental filter (name, type, tags) |
| `f` | Findings screen |
| `s` | Back to snapshot picker |
| `Esc` | Back / clear |
| `q` | Quit |

## Working with snapshots

```sh
azdocs snapshots list                 # all snapshots with counts
azdocs snapshots show latest          # per-query row counts, durations, errors
azdocs snapshots diff <a> <b>         # added/removed/changed resources
azdocs snapshots diff <a> latest --format json   # machine-readable, for CI
azdocs snapshots prune --keep 5 --yes
azdocs snapshots prune --older-than 90 --yes
```

Snapshot ids can be abbreviated to any unique prefix (e.g. `e4fb3710`).
`diff` compares resources by ARM id; `changed` means the properties JSON
differs.

## Ad-hoc queries

Run any KQL against Resource Graph without storing anything:

```sh
azdocs query list                       # the full pack + your custom queries
azdocs query show nsg_open_to_internet  # print the KQL
azdocs query run virtual_machines                      # by name
azdocs query run ./my-query.kql --format json          # from a file
echo 'resources | count' | azdocs query run -          # from stdin
azdocs query run subnets --format csv > subnets.csv
```

## Custom queries

Drop TOML files into the user queries directory (`azdocs query list` prints
the exact path — `~/Library/Application Support/azdocs/queries.d/` on macOS,
`~/.config/azdocs/queries.d/` on Linux). Same-named files **override**
built-ins; new names are added to the pack and run on every `collect`.

```toml
name = "expensive_vm_sizes"          # snake_case, unique
category = "compute"                 # any category; new ones create new report sections
kind = "inventory"                   # "inventory" or "finding"
description = "VMs using E-series sizes"
kql = '''
resources
| where type == "microsoft.compute/virtualmachines"
| where properties.hardwareProfile.vmSize startswith "Standard_E"
| project id, name, subscriptionId, vmSize = tostring(properties.hardwareProfile.vmSize)
| order by id asc
'''
```

Rules:

- **Finding queries** additionally need `severity = "high"|"medium"|"low"|"info"`
  and optionally `title_field = "<column>"` (the column used as the finding
  title; falls back to `name`, then `id`).
- End queries with `| order by id asc` (or another deterministic sort) —
  multi-page results are paginated with `$skipToken`, which needs stable
  ordering.
- Don't name a projected column `count` — it's a reserved word in KQL; ARG
  rejects it with an HTTP 400.
- Inventory query rows are stored verbatim and appear as tables in the
  category report pages; finding rows become severity-graded findings.

## Running in CI

Keep the secret out of the config file and use env vars:

```sh
export AZDOCS_TENANT_ID=... AZDOCS_CLIENT_ID=... AZDOCS_CLIENT_SECRET=...
azdocs init --non-interactive        # once, or ship a config without [auth]
azdocs collect --notes "$GIT_SHA"
azdocs report --format all
azdocs snapshots diff <baseline> latest --format json > drift.json
```

Exit codes are non-zero on failure, so `collect` failing wholesale fails the
job; a `partial` snapshot does not (check `snapshots show` output if you need
stricter behaviour).

## Troubleshooting

**`AADSTS7000215: Invalid client secret`** — the secret is wrong or expired.
Service principal secrets expire; create a new one with
`az ad sp credential reset --id <appId>`.

**`azdocs check` shows fewer subscriptions than expected** — the SP is missing
the Reader role on the absent subscriptions.

**`ARG throttled (429); backing off` warnings during collect** — normal on
larger tenants. Resource Graph allows bursts then throttles; azdocs paces
itself from the quota headers and retries with the server-provided delay. If
it bothers you, lower `--concurrency`. A snapshot only degrades to `partial`
if a query exhausts all its retries.

**`resource graph returned HTTP 400: BadRequest`** on a custom query — the
KQL is invalid for ARG (a stray reserved word like `count` is the classic).
Test it with `azdocs query run ./file.toml` before adding it to the pack.

**Reports look stale** — reports always come from a stored snapshot, not live
Azure. Run `azdocs collect` first; `--snapshot latest` (the default) uses the
newest complete/partial snapshot.

## Limitations

Azure Resource Graph only exposes control-plane configuration:

- No RBAC role assignments, no Entra ID objects.
- No data-plane state (blob contents, key vault secrets, SQL logins).
- No cost/billing data and no activity logs.
- `properties` for some types are partial compared to a direct `GET` on the
  resource (ARG serves a cached projection).

The audit is therefore a **configuration** audit, and read-only by
construction.
