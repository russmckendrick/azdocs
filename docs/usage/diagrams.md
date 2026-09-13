# Diagrams

```sh
azdocs diagram --type hierarchy|resources|network|vnets|resource-groups|workbook \
    [--format drawio|mermaid|both|svg|png|all] \
    [--subscription <id>] [--resource-group <name>] [--out <path>]
```

| Type | Shows |
|---|---|
| `hierarchy` | Tenant → subscriptions → resource groups with resource counts |
| `resources` | Resource-group containers, an icon per resource, attachment edges |
| `network` | VNets/subnets as containers, VMs placed in their subnets, dashed peering edges, NSG associations, private-endpoint links |
| `vnets` | One diagram per VNet: its subnets and resources, plus dashed stubs to peered VNets |
| `resource-groups` | One diagram per resource group: VNet subtrees homed there plus a "Not in a virtual network" zone |
| `workbook` | A single multi-sheet `.drawio` file: network topology, VNet peerings, then every per-VNet and per-RG sheet |

Diagrams exported here are drawn at **full detail** — every resource gets its
own icon and name, on whatever canvas the content needs. The same graphs
embedded in a report are **summarised** instead: resources aggregated by type
(`Storage Account ×13`) and the canvas snapped to a quarter, third, half or
full A4 portrait page, so they stay readable in print. See
[Diagram standards](../reference/diagrams.md) for the full contract.

`hierarchy`, `resources`, and `network` write one file
(`output/azdocs-<type>.<ext>`). The fan-out types write one file per scope
under `output/diagrams/vnets/` and `output/diagrams/resource-groups/`
(`--out` names the directory instead). The workbook writes
`output/azdocs-workbook.drawio` — or, for `svg`/`png`, one file per sheet under
`output/diagrams/workbook/`, since a raster cannot hold multiple sheets.

The desktop's Exports workspace writes the same layout under the directory you
pick, because both go through one function
(`commands::diagram::default_output_path`).

A `network` diagram is shaped like this (this is actual azdocs Mermaid output
style — GitHub renders it natively):

```mermaid
flowchart LR
    subgraph vnet_hub["vnet-hub<br/><i>10.0.0.0/16</i>"]
        subgraph snet_shared["shared<br/><i>10.0.1.0/24</i>"]
            vm1["vm-app-01<br/><i>Virtual Machine</i>"]
        end
    end
    subgraph vnet_app["vnet-app<br/><i>10.1.0.0/16</i>"]
        subgraph snet_app["app<br/><i>10.1.0.0/24</i>"]
            pe["pe-sql<br/><i>Private Endpoint</i>"]
        end
    end
    subgraph connected["Connected services"]
        sql["sql-prod<br/><i>SQL Server</i>"]
        nsg["nsg-app<br/><i>NSG</i>"]
    end
    vnet_hub -.-|Connected| vnet_app
    pe -.-|private link| sql
    nsg -.- snet_shared
```

Use `--tenant <reference-or-tenant-id>` to select the estate. Explicit snapshot
IDs remain usable offline without credentials; a supplied tenant selection
enforces ownership. See [tenant history](snapshots.md#tenant-isolation).

## Formats

- **`.drawio`** — editable in [draw.io](https://app.diagrams.net) or the
  VS Code draw.io extension. Uses the azure2 icon set, swimlane containers,
  and automatic edge routing. The only format the multi-sheet `workbook`
  type supports (with `--format all` it also rasterises each sheet).
- **`.mmd`** — Mermaid text. Paste into any GitHub markdown file, wiki, or
  docs site and it renders. Not available for `workbook` — use `vnets` /
  `resource-groups` for per-scope Mermaid.
- **`.svg`** — standalone vector render (shared layout with drawio). Also
  embedded in the HTML docs site (`output/docs-html/diagrams/`).
- **`.png`** — the SVG rasterised at 2× via resvg; needs no browser or
  external tool. SVG/PNG diagrams use the Microsoft Azure icon set embedded
  in the binary (mapping in `data/icon_mapping.toml`; unmapped types get
  the generic resource icon).

## Scoping

Estate-wide diagrams stop being useful past a certain size — over ~150 nodes
azdocs warns and suggests narrowing:

```sh
azdocs diagram --type network --subscription <id>
azdocs diagram --type resources --subscription <id> --resource-group rg-app
azdocs diagram --type vnets --format png            # one PNG per VNet
azdocs diagram --type workbook                      # everything, one drawio file
```

Next: [The TUI](tui.md)
