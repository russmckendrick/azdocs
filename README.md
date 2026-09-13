# azdocs

Audit, explore, and document an Azure estate from a desktop application or the
command line. `azdocs` runs a pack of Azure Resource Graph queries with a
**service principal with Reader access**, stores collected evidence locally in
SQLite snapshots, then lets you investigate resources, findings, and
relationships or export reports offline.

- **Reports**: Markdown docs tree, self-contained HTML and a multi-page HTML
  site, CSV, XLSX, plus branded PDF and DOCX assessments with optional technical
  references, rendered natively (no Chromium or Pandoc needed)
- **Diagrams**: draw.io (azure2 icons, editable), Mermaid, SVG, and PNG —
  estate hierarchy, per-subscription resource maps, network topology (VNets,
  subnets, peerings, NSGs, private endpoints), per-VNet and per-resource-group
  fan-outs, and a multi-sheet draw.io workbook of everything
- **Audit findings**: public blob access, NSGs open to the Internet,
  public database endpoints, disks using platform-managed encryption keys,
  missing required tags, orphaned resources, and more — graded by severity
- **Snapshots**: diff estates over time, prune old runs
- **Desktop explorer**: Tauri app with a subscription/resource-group tree,
  searchable resource ledger, property inspector, findings, snapshot history,
  and a Cytoscape.js Azure-icon estate map — all backed by the same local
  database
- **Website evidence**: desktop collection captures website screenshots using
  the platform webview and saves them with the snapshot for offline review
- **TUI**: browse the stored estate interactively (`azdocs browse`)
- **CLI portability**: one executable with bundled SQLite and no OpenSSL or
  Azure CLI runtime dependency. Desktop builds use the platform's webview.

## Documentation

Full documentation lives in [docs/](docs/README.md):

- [Usage](docs/usage/README.md) — install, configure, collect, explore, export, browse, CI
- [Development](docs/development/README.md) — architecture, data model, testing, contributing
- [Reference](docs/reference/README.md) — queries, operational evidence, themes, labels and diagram standards

## Install

Build from source with a current stable [Rust toolchain](https://rustup.rs/):

```sh
git clone https://github.com/russmckendrick/azdocs.git
cd azdocs
cargo install --path . --locked
```

This installs the CLI into Cargo's binary directory, which must be on your
`PATH`. See [Installation](docs/usage/installation.md) for platform requirements,
desktop builds and release-archive instructions. The project is preparing its
first release; use the source installation above until CLI archives appear on
the [Releases page](https://github.com/russmckendrick/azdocs/releases).

## Quick start

```sh
# 1. Create a service principal with Reader access (once)
# Replace the subscription ID; run as an identity allowed to create the app
# and assign Reader. Azure CLI is used for this provisioning step only.
az ad sp create-for-rbac --name azdocs-reader --role Reader \
    --scopes /subscriptions/<subscription-id>

# 2. Enter the returned tenant, appId and password
# Interactive secret entry saves to the OS credential store.
azdocs init

# 3. Verify connectivity
azdocs check

# 4. Collect a snapshot
azdocs collect

# 5. Export all report formats, including the PDF/Word technical references
azdocs report --format all --include-reference
# Export the network diagram as draw.io and Mermaid
azdocs diagram --type network --format both

# 6. Browse it
azdocs browse
```

Outputs default to `./output/`. For headless collection, use an
[environment secret reference](docs/usage/configuration.md#credentials-and-environment-variables).
Reader must be granted on each subscription or parent scope you intend to audit.

### Desktop

From the repository root, with Rust, Node.js 22.12+ (22.x), pnpm 10 and the
[Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/) installed:

```sh
cd desktop
pnpm install --frozen-lockfile
pnpm run tauri dev
```

Open **Settings → Add tenant**, test and save the connection, then choose
**Collect snapshot**. A CLI-created configuration can also be used. Desktop
installers are built from source; the release workflow packages the CLI only.
See the [desktop guide](docs/usage/desktop.md).

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

Set `AZDOCS_ACME_SECRET` in the process environment before a live check or
collection; the TOML stores the variable's name, not its value.

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

## Scope and limitations

The query pack covers accessible control-plane configuration and service
evidence exposed by Resource Graph, including policy, RBAC, Defender, backup
and patch records. Visibility, indexing delays and retention windows limit
coverage. A missing record or a successful empty query is not a passed control.
This is not a data-plane scan, a complete activity log, or a billing export.

Live preflight also inspects Azure RBAC assignments and role definitions through ARM; its
[permission verdict](docs/usage/permissions.md) is scoped evidence, not a
tenant-wide effective-access certification.

Desktop collection makes additional read-only Front Door management requests
and visits discovered website URLs to capture screenshots. Refreshing a
screenshot is also an online action. Reports, diagrams, snapshot browsing and
saved-image viewing use stored data only. See [website evidence](docs/usage/website-screenshots.md)
and [operational evidence](docs/reference/operational-evidence.md).

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked  # CLI + desktop backend; no Azure needed

cd desktop
pnpm install --frozen-lockfile
pnpm run lint
pnpm run typecheck
pnpm test
pnpm run build
```

Workspace checks need the desktop platform prerequisites. For CLI-only work,
use `cargo test --locked` and `cargo clippy --all-targets --locked -- -D warnings`.
See [Contributing](CONTRIBUTING.md) and [Testing](docs/development/testing.md)
for golden-file review, generated contracts and native smoke tests.

## Licence and security

azdocs code and documentation are available under the [MIT licence](LICENSE).
Bundled assets and adapted queries retain their upstream terms; see
[Third-party notices](THIRD_PARTY_NOTICES.md). azdocs is an independent project
and is not affiliated with or endorsed by Microsoft.

Report vulnerabilities privately using [SECURITY.md](SECURITY.md). Do not attach
credentials, collected databases or unsanitised estate reports to public issues.

## Credits

The desktop and bundled print fallbacks use [IBM Plex](https://github.com/IBM/plex)
under the SIL Open Font License 1.1. Field Report's PDF prefers installed
Charter, Arial and Courier New; Word names those fonts for its reader to resolve.
See [Fonts](docs/reference/themes.md#fonts). Azure service icons come from
[Microsoft's architecture icon set](https://learn.microsoft.com/en-us/azure/architecture/icons/).

Inspired by [billybeckett/Audit-Azure](https://github.com/billybeckett/Audit-Azure),
[adrian207/Audit-Azure](https://github.com/adrian207/Audit-Azure),
[dswann101164/azure-enterprise-diagram-automation](https://github.com/dswann101164/azure-enterprise-diagram-automation),
and Thomas Thornton's [draw.io MCP diagramming skill](https://thomasthornton.cloud/azure-diagram-agent-skill-with-draw-io-mcp/).
