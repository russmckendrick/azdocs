# azdocs documentation

azdocs audits an Azure estate with a read-only service principal, stores
point-in-time snapshots in SQLite, and exports reports and diagrams entirely
offline.

```mermaid
flowchart LR
    subgraph Azure
        ARG[Azure Resource Graph]
    end
    subgraph azdocs
        collect[azdocs collect]
        db[(SQLite<br/>snapshots)]
        report[azdocs report]
        diagram[azdocs diagram]
        browse[azdocs browse]
    end
    ARG -->|70 KQL queries<br/>read-only| collect
    collect --> db
    db --> report
    db --> diagram
    db --> browse
    report --> docs_out[Markdown · HTML · CSV · XLSX]
    diagram --> diag_out[draw.io · Mermaid]
    browse --> tui[Interactive TUI]
```

Everything right of the database works offline — reports and diagrams are
generated from the stored snapshot, never live Azure.

## Guides

### [Usage](usage/README.md)

| Page | Covers |
|---|---|
| [Installation](usage/installation.md) | Binaries, cargo install, shell completions |
| [Configuration](usage/configuration.md) | Service principal setup, config file, env vars |
| [Collecting](usage/collecting.md) | Running the query pack, scoping, throttling |
| [Reports](usage/reports.md) | Markdown, HTML, CSV, XLSX outputs |
| [Diagrams](usage/diagrams.md) | draw.io and Mermaid diagram types |
| [The TUI](usage/tui.md) | Browsing snapshots interactively |
| [Snapshots](usage/snapshots.md) | Listing, diffing, pruning |
| [Queries](usage/queries.md) | Ad-hoc KQL and custom query packs |
| [CI](usage/ci.md) | Running azdocs in pipelines |
| [Troubleshooting](usage/troubleshooting.md) | Common errors and limitations |

### [Development](development/README.md)

| Page | Covers |
|---|---|
| [Architecture](development/architecture.md) | Modules, data flow, design decisions |
| [Data model](development/data-model.md) | SQLite schema and migrations |
| [Testing](development/testing.md) | Test layers, fixtures, golden files |
| [Contributing](development/contributing.md) | Common tasks, style, crate gotchas |
| [Releasing](development/releasing.md) | CI and the release workflow |

### [Reference](reference/README.md)

| Page | Covers |
|---|---|
| [Query pack](reference/queries.md) | All 70 built-in queries and findings |
