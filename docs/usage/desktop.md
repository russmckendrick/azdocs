# Desktop explorer

The Tauri desktop app explores the same SQLite snapshots as the CLI and TUI.
Its relationship view is an interactive Cytoscape.js canvas whose graphs are
built by the Rust side (`topology_graph`) with one non-negotiable rule:
**every resource in scope is represented** — drawn as its own node, folded
into its host (NICs and disks into their VM, child resources into their ARM
parent), or aggregated into a ×N tile — and the status chip states the exact
arithmetic. Nothing is silently truncated. Resource nodes are draggable, the
canvas supports pan and zoom, and directional flow motion is optional. Each
link receives its own invisible source and target boundary ports: shared
endpoints fan into stable, spatially ordered taxi channels instead of sharing
one trunk. Relationship names are DOM label plates above the canvas, so no
connector can paint over their text.
Every resource uses the vendored Microsoft Azure service icon resolved for its
ARM type.
It does not maintain a second inventory and it does not query Azure while you
browse: resource details, findings, relationships, history, and comparisons
all come from the selected local snapshot.

Website endpoints also have [saved screenshots](website-screenshots.md), captured automatically after desktop collection or on request. Browsing saved images stays offline.

## Run from a source checkout

Node.js, Rust, and the [Tauri v2 platform prerequisites](https://v2.tauri.app/start/prerequisites/)
for your operating system are required.

```sh
cd desktop
pnpm install
pnpm run tauri dev
```

Build web assets without opening a native window:

```sh
cd desktop
pnpm run build
```

Create a platform installer or application bundle:

```sh
cd desktop
pnpm run tauri build
```

## What you can explore

- **Estate** keeps the subscription/resource-group hierarchy, searchable
  resource ledger, and resource inspector visible together. Filter by scope,
  type, location, resource name, ARM type, group, or tags.
- **Map** is a full-bleed, hierarchical graph workspace. Its initial
  camera fits the readable connected core; **Fit all** is the first control in
  the icon ribbon when you need the complete overview. The **estate** level
  opens with every subscription collapsed to a bar in a strip across the
  top; expand one and it is laid out below as a full-width grid of the
  resource groups that have cross-group relationships. Groups nothing crosses
  into share one **Unconnected groups ×N** tile per subscription, so the
  connectors are the picture rather than a wall of boxes; the tile expands in
  place to list its groups, and the **Unconnected** toggle hides it (the
  hidden count stays on the status rail). Click or press Enter on a group to
  open its group map immediately.
  A group map places contained and connected resources first, neighbours from
  other groups on boundary rails, and resources without drawn relationships in
  a lower secondary region outside the default camera. VNet → subnet
  containment remains framed, NICs/disks remain folded into their VM, child
  resources remain folded into their parent, and fan-outs remain represented
  by ×N tiles. The **neighbourhood** mode is available only after a resource is
  explicitly selected: that resource is centred, inbound relationships are on
  the left, outbound relationships on the right, and second-hop nodes occupy
  deterministic outer columns.

  Zoom is semantic: full labels at readable scale, primary labels at the
  middle rung, then icon/count overview tiles. Text is never rendered below
  11px and relationship labels appear only on the active traced path. Keyboard
  focus wins over pointer hover, which wins over the selected neighbourhood
  subject; unrelated routes fade without disappearing. Each active label sits
  beside the remote endpoint on an opaque plate and moves with pan, zoom,
  drag, resize, and camera animation. The camera
  adapts its fit to the graph level and available viewport: sparse group and
  neighbourhood maps may zoom above 1:1, while Fit all remains an overview.
  Scope changes and drill actions use Cytoscape's native viewport animation
  and become instant when reduced motion is active. Click or press Enter on a
  resource to open its resource record. Use the labelled relationship-count
  action on the card, or press **R** while the card has keyboard focus, to open
  its neighbourhood directly. The record repeats this as **Explore N
  relationships**. Back returns through the exact group, record, and
  neighbourhood history, while the trail in the title bar — Map ›
  subscription › group › resource, ending in the current page's own name —
  moves directly through the hierarchy: the subscription crumb returns to
  the map with that subscription expanded, and the group crumb opens its
  group map. Aggregate tiles expand in place to expose
  their members. Spatial arrow keys move between graph
  items. The joined icon ribbon exposes Fit all, zoom, selected-path motion,
  subscription-lane reset, and help with visible tooltips and accessible names.
  The slim bottom rail reports how many groups or resources are drawn
  and where the rest went (collapsed subscriptions, unconnected tiles, folded
  hosts, filters), then how many **relationships** are drawn as how many
  **connectors** — the two differ because parallel relationships between the
  same pair share one connector — and carries the relationship-kind
  legend/filter. Anything hidden by a filter remains counted. If a topology request fails, the last successful graph is retained
  and identified as stale with Retry and Revert controls. Relationships remain
  Rust post-pass edges already stored in SQLite; opening this view does not add
  API calls.
- **Findings** orders stored audit evidence by severity and links every
  resource-scoped result back to its resource properties.
- **Snapshots** shows collection history, query health, and added, changed, or
  removed resource IDs compared with the preceding snapshot.
- **Exports** generates four outcome-led artifacts from the active snapshot:
  the print-ready PDF Field Report, an editable Word Report, the Excel Data
  Workbook, or the Draw.io Diagram Workbook. The app deliberately omits format
  matrices and theme choice; HTML, CSV, Markdown, scoped diagrams, Mermaid,
  SVG and PNG remain available to CLI and automation users.

The global search shortcut is `Cmd+K` on macOS or `Ctrl+K` on Windows and
Linux. Every resource pane and navigation action is keyboard reachable.

## Data and credentials

The app initially uses `storage.db_path` from the normal azdocs configuration.
**Open data** can switch to another existing `.db`, `.sqlite`, or `.sqlite3`
file for the current session. The chosen database is opened by Rust; the
webview never receives a file-system permission.

**Collect snapshot** invokes the existing client-credentials and Azure
Resource Graph pipeline. Credentials remain in the config/environment on the
Rust side and are never sent through Tauri IPC. If credentials are not fully
configured, the button is disabled and its tooltip points at the resolved
configuration path.

The collector is the only networked path:

```mermaid
flowchart LR
    ui[Tauri webview] -->|typed command| rust[Rust desktop boundary]
    rust --> config[azdocs config + credentials]
    rust --> collect[existing collect pipeline]
    collect -->|read-only| arg[Azure Resource Graph]
    collect --> db[(shared SQLite database)]
    db -->|typed snapshot DTO| rust
    rust --> ui
```

All other desktop commands open the selected database for one request and
return serialisable snapshot data. The SQLite connection is not shared across
webview commands.

## Offline exports

The **Exports** workspace uses a native directory picker; the selected path is
sent to Rust only for the duration of the export. The webview receives progress
and a sorted manifest of completed files, but it never receives general file
system access. Existing configuration still supplies report branding, custom
themes, logos, fonts and labels. The app's own wording comes from the same
`[branding] labels` set, resolved once at startup — see
[reference/labels.md](../reference/labels.md).

Reports compose shared diagram assets once even when several formats are
selected. Per-VNet and per-resource-group diagrams fan out beneath the chosen
directory, while the workbook writes an editable `.drawio` file or one SVG/PNG
per sheet. Every export reads only the selected SQLite snapshot and does not
contact Azure.

## Browser preview

`pnpm run dev` opens the web interface without Tauri. In that mode the app uses
the canonical fixture-shaped illustrative estate and labels the toolbar
**Illustrative workspace**. This is for responsive and visual development;
only a Tauri window reads real databases or collects Azure data.

Next: [Snapshots](snapshots.md)
