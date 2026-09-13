# azdocs

Audit, explore, and document an Azure estate from a desktop application or the
command line. `azdocs` runs a pack of Azure Resource Graph queries with a
**read-only service principal**, stores everything locally in SQLite as
point-in-time snapshots, then lets you investigate resources, findings, and
relationships or export the whole estate — entirely offline once collected.

- **Reports**: Markdown docs tree, self-contained HTML, CSV, XLSX, plus
  branded PDF and DOCX rendered natively (no Chromium or Pandoc needed)
- **Diagrams**: draw.io (azure2 icons, editable), Mermaid, SVG, and PNG —
  estate hierarchy, per-subscription resource maps, network topology (VNets,
  subnets, peerings, NSGs, private endpoints), per-VNet and per-resource-group
  fan-outs, and a multi-sheet draw.io workbook of everything
- **Security findings**: public blob access, NSGs open to the Internet,
  public database endpoints, unencrypted disks, missing required tags, orphaned
  resources, and more — severity-graded
- **Snapshots**: diff estates over time, prune old runs
- **Desktop explorer**: Tauri app with a subscription/resource-group tree,
  searchable resource ledger, property inspector, findings, snapshot history,
  and a Cytoscape.js Azure-icon estate map — all backed by the same local
  database
- **TUI**: browse the stored estate interactively (`azdocs browse`)
- Single static binary for macOS, Linux, and Windows; no OpenSSL, no system
  SQLite, no Azure CLI required

## Documentation

Full documentation lives in [docs/](docs/README.md):

- [Usage](docs/usage/README.md) — install, configure, collect, explore, export, browse, CI
- [Development](docs/development/README.md) — architecture, data model, testing, contributing
- [Reference](docs/reference/README.md) — the built-in query pack

## Quick start

```sh
# 1. Create a read-only service principal (once)
az ad sp create-for-rbac --name azdocs-reader --role Reader \
    --scopes /subscriptions/<subscription-id>

# 2. Write the config (interactive; secret can stay in the environment)
azdocs init

# 3. Verify connectivity
azdocs check

# 4. Collect a snapshot
azdocs collect

# 5. Export everything
azdocs report --format all
azdocs diagram --type network --format both

# 6. Browse it
azdocs browse

# Or launch the desktop explorer from a source checkout
cd desktop
pnpm install
pnpm run tauri dev
```

## Configuration

`azdocs.toml` (searched in the current directory, then the platform config
dir; `--config` overrides):

```toml
schema_version = 2
default_tenant = "acme"

[tenants.acme]
name = "Acme"
tenant_id = "11111111-1111-4111-8111-111111111111"
client_id = "22222222-2222-4222-8222-222222222222"
secret_env = "AZDOCS_ACME_SECRET"

[audit]
required_tags = ["environment", "owner"]
```

The desktop **Settings** editor manages named tenants, shared defaults,
branding and application preferences. New secrets use the OS credential store;
headless profiles can reference environment variables explicitly. Legacy
`[auth]` files and environment-only operation remain supported. Use Settings
or `azdocs config migrate` to migrate a legacy file with a protected backup.

Choose a tenant with `--tenant acme` or the desktop toolbar. One shared SQLite
database keeps each tenant's history isolated. [Configuration](docs/usage/configuration.md)
covers inheritance, selection, secret storage and migration.

## Commands

| Command | Purpose |
|---|---|
| `azdocs init` | Write the config file interactively |
| `azdocs check` | Test authentication, subscriptions and advisory RBAC permissions |
| `azdocs config show / validate / migrate / set-secret` | Inspect, validate, migrate or update secure configuration |
| `azdocs collect` | Run the query pack into a new snapshot |
| `azdocs snapshots list/show/diff/prune` | Manage stored snapshots |
| `azdocs report --format md\|html\|csv\|xlsx\|pdf\|docx\|all` | Export reports |
| `azdocs diagram --type hierarchy\|resources\|network\|vnets\|resource-groups\|workbook` | Export diagrams |
| `azdocs query list/show/run` | Inspect and run individual KQL queries |
| `azdocs browse` | Interactive TUI over a stored snapshot |
| `azdocs completions <shell>` | Shell completions |

## Custom queries

Drop TOML files into `<config dir>/azdocs/queries.d/` to add queries or
override built-ins by name (`azdocs query list` prints the exact path):

```toml
name = "expensive_vm_sizes"
category = "compute"
kind = "inventory"          # or "finding" (requires severity)
description = "VMs using expensive size families"
kql = '''
resources
| where type == "microsoft.compute/virtualmachines"
| where properties.hardwareProfile.vmSize startswith "Standard_E"
| project id, name, subscriptionId, vmSize = tostring(properties.hardwareProfile.vmSize)
| order by id asc
'''
```

Multi-page ARG results require a deterministic sort — end custom queries with
`| order by id asc`.

## What Resource Graph can't see

Inventory auditing covers control-plane configuration. Live preflight also
inspects Azure RBAC assignments and role definitions through ARM; its
[permission verdict](docs/usage/permissions.md) is scoped evidence, not a
tenant-wide effective-access certification.

## Development

```sh
cargo test          # unit + integration + golden-file tests (no Azure needed)
cargo insta review  # accept intentional report/diagram output changes

cd desktop
pnpm install
pnpm run build       # TypeScript + production web assets
pnpm run tauri dev   # desktop app with the shared Rust core
```

## Credits

Reports are set in [IBM Plex](https://github.com/IBM/plex) (Sans and Mono),
vendored under `data/fonts/` and used under the SIL Open Font License 1.1.
Diagram icons come from the Microsoft Azure icon set under `data/icons/`.

Inspired by [billybeckett/Audit-Azure](https://github.com/billybeckett/Audit-Azure),
[adrian207/Audit-Azure](https://github.com/adrian207/Audit-Azure),
[dswann101164/azure-enterprise-diagram-automation](https://github.com/dswann101164/azure-enterprise-diagram-automation),
and Thomas Thornton's [draw.io MCP diagramming skill](https://thomasthornton.cloud/azure-diagram-agent-skill-with-draw-io-mcp/).
