# Desktop design language

The desktop is a premium desktop data application, not an Azure Portal clone:
dense technical information presented with calm visual hierarchy, an
environment you can spend several hours in — polished, fast,
information-rich, never visually noisy. Reduced to a formula:

> Dark navy application frame + cool neutral workspace + restrained Azure
> blue + excellent typography + subtle depth + highly structured technical
> data.

And in dark mode:

> Near-black navy canvas + layered blue-grey surfaces + bright but restrained
> Azure highlights + almost no decorative glow.

This palette belongs to the desktop app only; PDF and DOCX continue to use the
selected data-driven document theme. The tokens live in
[`desktop/src/styles.css`](../../desktop/src/styles.css) — the only place a
colour value is written down. [design.html](design.html) shows them as
swatches, alongside the type scale and the brand marks; it is generated from
those sources by
[`tools/build_design_sheet.py`](tools/build_design_sheet.py), so never edit it
by hand.

| Quality | What it means in the interface |
|---|---|
| **Technical** | Precise data, compact controls, Azure resource terminology, meaningful diagrams |
| **Calm** | Soft surfaces, restrained colour, generous but not wasteful spacing |
| **Premium** | Excellent typography, subtle depth, smooth states, careful alignment |
| **Trustworthy** | Data is timestamped, states are explicit, findings are never hidden behind effects |

It should not look like the Azure Portal, a Bootstrap admin template, a
neon security dashboard, a SaaS marketing dashboard, or a pile of
independent cards with no overall structure.

Rules that matter, straight from review:

- **Selection is a quiet tinted fill** (`--az-selected`, and the soft blue
  row in the navigation frame) — never a coloured bar or left-edge accent.
- **No numbering chrome.** Figures end with a short caption, never
  "Fig. 3.1". Headings are names, not chapter numbers.
- **Colour is semantic.** Azure blue marks primary actions, selected
  navigation, links, interactive series and the selected object; it does not
  colour every heading or icon. Saturated colour is otherwise reserved for
  data (the category set) and signals (the severity set), and each supporting
  accent stands for one thing everywhere.
- **Nothing below 11px** at the default text scale.
- **The app stays dense.** Polish comes from hierarchy, surfaces and
  alignment, not from whitespace; the design optimises for scanning and
  comparison across hundreds of resources, findings and relationships.

## Frame

Three persistent layers:

| Layer | Purpose |
|---|---|
| Navigation frame | Stable app-level navigation, dark navy in both modes |
| Command bar | Tenant, snapshot, search and the two global actions |
| Workspace | Page-specific content, scrolling independently |

| Element | Size |
|---|---:|
| Sidebar expanded | 208px |
| Sidebar collapsed | 64px (80px under the macOS overlay title bar) |
| Command bar | 56px |
| Workspace padding | 24px horizontal, 20px vertical |
| Card gutter | 12px |
| Major section gap | 16–20px |

The desktop targets 1440×900 (design reference 1536×960) and a working
minimum of 1280×720. Content stretches until about 1840px of workspace,
after which the workspace gains outer margins rather than stretching charts
indefinitely. Below 1150px the command bar's context selectors shrink first;
below 800px the sidebar drops to its rail.

**Window chrome is native.** On macOS `tauri.conf.json` sets
`titleBarStyle: Overlay` with a hidden title, so the traffic lights float
over the navy frame; the shell marks `<html data-titlebar="overlay">` and
the stylesheet pads the frame by `--titlebar-inset` (32px) and widens the
collapsed rail to clear the light cluster. The brand row and command bar are
`data-tauri-drag-region`, which needs the `core:window:allow-start-dragging`
capability. Windows and Linux keep their own controls and the same layout;
no platform control is ever drawn by an application component.

### Navigation frame

The frame is the app's permanent visual anchor. Its header is the reversed
mark at 28px beside the product name (15px / 650) over the descriptor
(11px, muted). Navigation rows are 36px tall with a 10px inset, an 18px
outline icon (Lucide — one unified set for UI controls) and a 12px gap;
default is transparent with muted white text, hover a soft lift, the
selected row a soft blue emphasis in white text. The Findings count sits
beside its label as a quiet badge, visually subordinate to the word. The
footer holds Settings; the persistent collection state (a coloured dot, the
snapshot status, database path and version) lives on the status bar beneath
the workspace, so it is never only a toast and never shown twice. The
collapse control lives at the left edge of the command bar, where native
macOS toolbars keep it; the collapsed rail keeps icons, tooltips, accessible
names and a dot for the findings badge.

Azure artwork does **not** appear in the navigation frame. It belongs in the
workspace, where it identifies real subscriptions, groups and resources.

### Command bar

From left to right: the sidebar toggle, the tenant selector, the snapshot
selector, search, then *Open data* (secondary) and *Collect snapshot* (the one
primary action). The two selectors are compact stacked controls — a tracked
11px label (`TENANT`, `SNAPSHOT`) over the value — and always remain
visible. Search is the widest control, carries the `⌘K` / `Ctrl K` hint and
presents its results as a command palette: a group heading, then rows with
the resource icon, name and type. Keyboard navigation is essential and
already works; grouped results for groups, subscriptions, tags, findings and
commands are the intended direction.

## Modes

Theme is a tri-state in Settings: **System** (default — follows the OS via
`prefers-color-scheme`), Light, Dark. An explicit choice pins
`data-theme="light|dark"` on `<html>` and persists in `localStorage`
(`azdocs-theme`); System removes the attribute. Switching is immediate.

Dark mode is a genuine re-tuning of the colour relationships rather than an
inversion: matte, layered and quiet, with only interactive or selected
elements becoming vivid. Borders replace shadows as the carrier of depth
(cards have none), the primary blue brightens so it does not sink into the
canvas, and series colours rise slightly in luminance without going neon.
Every light token has exactly one dark counterpart in the two dark blocks of
`styles.css`, which the design-sheet generator requires to agree. Components
never branch on the mode; charts, SVGs, the topology canvas and code panels
all consume the same tokens.

## Tokens

Values belong to `styles.css`; the generated [design sheet](design.html)
shows the light and dark swatches.

| Token | Role |
|---|---|
| `--az-canvas` | workspace ground — never pure white; `--az-canvas-glow` is the barely-there cool wash near the header |
| `--az-surface-1` … `-3` | cards and the command bar; raised controls; evidence blocks, tracks and chips |
| `--az-hover` / `--az-selected` | rows under the pointer; the selected row, tab, tree node or control |
| `--az-text-primary` / `-secondary` / `-muted` | headings, values and running text; supporting copy, icons and captions; placeholders and micro metadata |
| `--az-border` / `-subtle` / `-strong` | card and control boundaries; row separators; hover and focus boundaries |
| `--az-primary` (`-hover`, `-active`, `-soft`) | Azure blue for links, selected navigation, series and focus |
| `--az-primary-fill` (`-hover`, `-active`) + `--az-on-primary` | filled buttons — the light-mode blue in both modes so the white label reads |
| `--az-success` / `-warning` / `-danger` / `-info` / `-neutral` | signals, always paired with a word or icon |
| `--severity-high` … `-info` | the severity set, aliased onto the signals |
| `--az-resource` / `-governance` / `-relationship` | supporting accents: KPI tints for resources, tags and stored connections |
| `--az-sidebar` (`-elevated`, `-hover`, `-selected`, `-text`, `-text-strong`, `-muted`, `-border`, `-badge`, `-focus`) | the navigation frame |
| `--cat-*`, `--kind-*` | the category chart set and relationship kind classes |
| `--graph-*`, `--chart-*`, `--map-land` | the topology plate, chart grid and axis text, the understated map land |
| `--space-*`, `--r-*`, `--control-h*`, `--motion`, `--ease` | the 4px spacing scale in rem, radii, control heights, motion |

### Category / chart set

Fixed order, never cycled — the top slots of the app's Rust
`category_color()` set. Charts fold the tail into *Other*; type dots
elsewhere may use the full set. Check both mode palettes against their
surfaces and with colour-vision deficiency simulation when changing them;
labels must retain the meaning without colour. Only the storage slot changes
in dark. Relationship kind classes (`--kind-network` green, `structure`
blue, `data` violet, `identity` orange-red, `monitoring` amber) map onto the
same set.

### Severity

High = danger, medium = warning, low = info, info = neutral. Always
combine icon, colour and the word or count; the severity counts also exist
in text outside every chart. Rows referenced by open findings carry a
dagger (†) in every table.

## Type

Inter Variable is bundled from `@fontsource-variable/inter` and is the whole
interface face; machine values (ARM ids, subscription ids, hashes, raw
values) use the platform monospace stack with IBM Plex Mono served from
`data/fonts/` as the cross-platform fallback. The PDF keeps its own faces
(see [Fonts](themes.md#fonts)); IBM Plex Serif stays vendored in
`desktop/src/assets/fonts/` only because `src/report/pdf.rs` embeds it.

| Role | Size | Weight | Line height |
|---|---:|---:|---:|
| Page title | 24px | 650 | 1.2 |
| Section heading | 16px | 650 | 1.35 |
| Card heading | 14px | 600 | 1.3 |
| KPI value | 26px | 650 | 1.15, tabular numerals |
| Body | 13px | 400 | 1.4 |
| UI control | 12px | 500 | 1.4 |
| Secondary | 11px | 400 | 1.4 |
| Label | 11px | 600 | tracked `0.08em` caps |
| Mono | 11px | 400 | ARM ids, tags, deltas, query names |

This is one step below the ramp the specification proposed (30/18/15/32/
14/13/12/11); the larger sizes read oversized on the real app and the reader
asked for a denser interface. Type and spacing are written in `rem` off a
12px base (`html { font-size: calc(75% * var(--ui-scale)) }`), so `Cmd`/`Ctrl`
`+`, `−` and `0` step the whole interface through 100–150% together while
hairlines, radii, fixed sizes and the graph canvas stay in `px`. Uppercase is
acceptable only for compact metadata labels such as `TENANT` and `SNAPSHOT`,
never for navigation.

## Structure

- **Spacing** uses a 4px base: `4 · 8 · 12 · 16 · 20 · 24 · 32 · 40 · 48`
  (`--space-1` … `--space-12`). Avoid arbitrary values unless a native
  control dictates them.
- **Radii**: 5px badges and chips, 7px inputs, buttons and dropdowns, 8px
  menus, 10px cards, 12px large panels, 14px dialogs. Pills only for true
  statuses, filters and tags.
- **Depth**: cards sit only slightly above the canvas — surface-1, one
  `--az-border` hairline, a 10px radius and a whisper of shadow in light
  (`--az-shadow-card`), none in dark. Menus and dialogs use
  `--az-shadow-pop`. Rules are 1px hairlines; there are no heavy ink rules.
- **Controls** are 32px tall by default (28px compact, 36px prominent), with
  a 7px radius, one hairline and 12–14px text. Dropdown menus use an 8px
  radius, 6px internal padding and 32–36px rows.
- **Motion** is subtle: 120–180ms with `cubic-bezier(.2,.8,.2,1)`, for
  panel opening, hover states, navigation selection, topology zoom and menu
  appearance. KPI values and charts are never animated on page entry; they
  ease only when their data changes. Reduced motion removes the animations.
- **Focus** is `2px solid` Azure blue with a 2px offset (`#2997ff` on the
  navy frame), never only a background change.

### One surface per route

Cards are the Overview's idiom; the rest of the application does not become
cards everywhere.

| Screen | Primary UI |
|---|---|
| Overview | analytical dashboard: KPI cards and chart panels |
| Estate | subscription tree beside a resource table |
| Map | topology canvas |
| Regions | full-page world map of every Azure datacentre, the snapshot's regions highlighted, above a region table |
| Inventory | category rail beside a query table |
| Findings | severity tally, filters, findings list and an evidence drawer |
| Governance | tagging analysis: stat strip, meters and a compliance ledger |
| Changes | snapshot ledger beside a comparison |
| Exports | deliverable list beside the run panel |
| Resource record | linear evidence sheet with compact header facts |

Estate keeps its hierarchy rail because scope is part of browsing; the rail
is part of the workspace (no borders — indentation carries the hierarchy,
counts align right), not a second application sidebar. A resource opens as a
dedicated evidence sheet: all stored properties, SKU, identity, tags,
findings, edge properties, ARM identifiers and provenance rendered without
tabs or truncation, through a shape-driven evidence renderer that presents
scalar and nested fields as compact property rows and collections as real
tables. Location, kind, finding, relationship and tag summaries occupy
compact header badges. Finding evidence remains an on-demand drawer.
Relationships navigate directly from groups to group maps and from resources
to their records; Back restores the preceding record or graph scope, while
breadcrumbs navigate the estate/group/resource hierarchy. Large collections
use labelled selectors and honest progressive batches; lists and tables own
their scroll area.

## Components

- **Buttons** come in three levels: primary (Azure blue fill, white label —
  one per surface, so nothing competes with *Collect snapshot*), secondary
  (bordered neutral, e.g. *Open data*) and tertiary (text with an arrow, e.g.
  *View inventory →*).
- **Tables** are clean and flat: 12px semibold muted headers, 48–56px rows,
  `--az-hover` under the pointer, `--az-selected` when selected, and never a
  rounded rectangle per row. The resource name cell stacks the Azure icon
  (20px), the name and the human-readable type; the raw Microsoft type
  appears on the record. The **Signals** column surfaces concrete counts
  (findings, relationships) with tooltips, or the subdued word *Quiet*.
- **Filters** occupy a dedicated row above a dataset as 36px selects; a
  *Clear filters* text action appears only once something is applied.
- **Search results** are a command palette: 8px radius, a tracked group
  heading, 36px rows, the active row in `--az-selected`.
- **Tag chips** are mono on `--az-surface-3` with a 5px radius; a missing
  required tag is a dashed danger outline. Header badges (location, kind,
  counts) share the 5px badge radius.
- **Evidence blocks** (stored JSON, ARM ids) are mono on `--az-surface-3`
  with a subtle hairline — never a dark backplate in light mode.
- **Tooltips** (native `title`, and the topology ribbon's own) cover
  truncated names, icon-only buttons, relationship types, chart points, Azure
  types and signal counts; essential information never lives only there.
- **Loading** uses skeletons that sketch the dashboard; **collection** shows
  real activity — stage, query progress, rows, failures and the latest query
  — in the collection dialog. **Empty** and **error** states stay technical:
  what happened, what was affected and whether the collected data remains
  usable, with no illustrations.

### Azure iconography

Official resource icons identify Azure entities everywhere in the workspace
and are never overpowering: 20px in table rows, 22px in cards and KPI
containers, 22–28px on the topology canvas, 32px on a record's header. UI
controls use Lucide outline icons; Fluent, Font Awesome, emoji and custom
cartoon icons are not mixed into the same context.

## Overview dashboard

Overview answers "what does this Azure estate look like right now" in one
scrolling document. Four equally weighted KPI cards lead — resources, audit
findings, any-tag coverage and stored relationships — each with its Azure
icon in a 32px soft-tinted container (8px radius; cyan for resources, blue or
danger for findings, purple for tag coverage, violet for relationships —
pastel in light, a low-opacity wash in dark), the metric label beside it, the
value at 32px and a supporting line with a chevron. Only a findings value
with high-severity results uses the danger colour.

Charts feel integrated rather than pasted from a library: no chart borders,
faint gridlines, compact axes, direct labels where practical and tooltips on
hover, with legends in the matching semantic colours. There is no page header
above the dashboard — the dashboard is the orientation — only a compact row
with the subscription scope and the snapshot stamp. The responsive
twelve-column grid leads with the resource-locations map (small pulsing Azure-blue markers on
an understated world map) in the wide panel beside the findings-by-severity
donut (the total and "Audit findings" at its
centre, a named, counted legend beside it); the map's direct-labelled region
list sits alongside it. Resource composition (horizontal bars with the Azure
icon, label and count), resource history (a 2px Azure line over a faint area;
points enlarge on hover, and partial or failed snapshots are marked in the
warning colour with their status) and tag coverage by subscription (the
percentage leads each row so the list scans) follow as thirds, then
collection coverage, changes since the previous snapshot and priority checks.
Below 950px the middle panels rebalance; below 650px the KPI cards form two
columns and panels stack. Labels stay at least 11px.

The subscription selector scopes current resource counts, findings, types,
regions and tag coverage. Relationship counts include connections touching
either endpoint in that scope. Findings without a resource remain estate-level
and cannot be assigned to a selected subscription. The following measures
explicitly retain whole-snapshot scope across all collected subscriptions:

- **Resource history** includes the same tenant's stored snapshots up to the
  selected snapshot, within 30 days, 90 days or all time. Running collections
  are excluded. Partial and failed collections stay visible as labelled gaps;
  neither the line nor its area fill connects across them. Only completed
  totals set the padded, automatically scaled axis. Ticks are distinct full
  localised integers, never rounded compact labels, and the left margin grows
  with their width. The scale need not start at zero; the visible axis and
  scope note make that clear. Collection scope may vary between snapshots.
- **Collection coverage** counts successful query runs against attempted runs
  by category. It measures collection evidence, not compliance.
- **Changes** use the stored comparison only when both snapshots are complete
  and belong to the same tenant. Additions, removals and property changes keep
  their separate counts; missing comparison evidence has an explicit empty state.

Tag coverage means resources with any tag. Required-tag compliance belongs in
Governance. Rust-provided percentages and governance verdicts remain
authoritative; the dashboard does not define thresholds or reround verdicts.

KPI cards, chart values, snapshot points and priority checks open a focused
native `dialog` (14px radius, `--az-shadow-pop`, a navy scrim). Its heading
names the selection and scope, the body shows matching stored records in
progressive batches, and the footer links to the exact result filters or
selected snapshot. Resource and relationship rows provide direct record or
neighbourhood navigation. The dialog has a labelled close button, native
focus containment, Escape and backdrop dismissal, and a scrolling body that
leaves the header and actions accessible. Result pages show the originating
selection, a return action and a clear-filter control.

KPI numbers interpolate only when their value changes, bars and donut
segments ease to new values, and the dialog and backdrop enter together;
nothing replays when the reader merely enters the page. Sidebar width changes
immediately; labels fade and slide in on expansion. Reduced motion removes
these animations and applies numeric changes directly.

The implementation is in
[`OverviewView.tsx`](../../desktop/src/components/OverviewView.tsx),
[`DashboardModal.tsx`](../../desktop/src/components/DashboardModal.tsx),
[`dashboard-model.ts`](../../desktop/src/components/dashboard-model.ts) and
[`dashboard-motion.ts`](../../desktop/src/components/dashboard-motion.ts).
Result navigation follows
[`navigation-state.ts`](../../desktop/src/navigation-state.ts); all wording
continues to come from the typed [labels](labels.md).

The locations map draws land in `--map-land` on the card, so the water is the
card itself, and never becomes a heat map. The outline is Natural Earth's
1:110m land layer (public domain), simplified to about three thousand points
by `desktop/scripts/build-land-outline.mjs` into the generated
[`land-outline.ts`](../../desktop/src/components/land-outline.ts) and projected
at render time with the Natural Earth I projection in
[`world-map.ts`](../../desktop/src/components/world-map.ts). Region
coordinates come from `data/azure_regions.toml`, generated with the display
names by `cargo run --example update_azure_locations` from Microsoft's Azure
regions list and datacenter map (see
[Azure metadata](../development/azure-metadata.md)); they reach the desktop as
`azureMetadata.regions`. A region without coordinates is still counted in the
list beneath the map, and every marker has a tooltip, an accessible name and
opens the same detail dialog as the list. *Browse all regions* opens the
Regions page rather than a dialog listing every resource.

### Regions

Regions is the map at full width: every datacentre in the catalogue as a
quiet hollow dot (its name and physical location on hover) and the regions
this snapshot uses as small pulsing Azure-blue markers, each labelled with its
name and count (size never encodes the count — a bigger circle beside a
smaller neighbour read as the wrong region). Labels prefer the right of their
marker, then a row down or up on the same side, and only then the left or
above/below, so neighbouring regions such as UK South and West Europe do not
swap; a label off the default side is joined to its marker by a leader line,
and a surface-coloured halo keeps text legible over coastlines. The map
carries a joined zoom ribbon in the corner (zoom in, zoom out, reset) and pans
by dragging once zoomed; the land scales about the frame while markers and
labels are re-projected through the camera, so they keep their pixel size and
labels re-place themselves as room appears. A faint coastline stroke (`--map-coast`) defines the land in both modes. The
land is projected once per width change into pixel space, so labels and
strokes never scale with the map. Neighbouring datacentres (London and Cardiff, the Virginia pair) sit closer
than any comfortable hit target, so clicks on the map resolve to the nearest
marker centre within 12px rather than to overlapping target shapes; the
marker groups keep keyboard activation. Any marker — in use or not — opens a
details dialog: the programmatic name,
physical location, geography, coordinates, availability-zone support, paired
region, opening year, open or announced status, Microsoft's data-residency
statement and the snapshot's count there, with *Open resources* for a region
in use. Beneath the map, a legend and a flat table list each active region
with its physical location, geography, count and share; a row opens the
Estate filtered to that region, which is the useful answer to "what is
there". The Overview's small map stays a static picture.

### Inventory evidence disclosures

Inventory keeps operational summaries and each query's recorded provenance in
native disclosures, collapsed until requested. Rust owns the summary analysis
and labelled wording; the frontend renders the supplied statuses, notes,
columns and cells. Evidence semantics, source limits and historical provenance
are documented in [Operational and access evidence](operational-evidence.md).

Expanded disclosures contain bounded scrolling panels (320px maximum height).
Long source URLs, query hashes and saved KQL wrap within the available width;
KQL retains its line breaks and mono face. The query record is keyboard
focusable and scrolls, and the outer workspace also scrolls when disclosures
exceed a short window. Preserve the raw table's minimum viewport (160px) so
opening provenance cannot collapse the ledger or make its footer unreachable.
These behaviours live in
[`InventoryView.tsx`](../../desktop/src/components/InventoryView.tsx) and the
inventory rules in [`styles.css`](../../desktop/src/styles.css).

### Map workspace

Map alone is full-bleed and reads as a technical canvas: the plate is
`--graph-canvas` with a barely perceptible 1px grid at 32px spacing
(`--graph-grid`, about 3% opacity). One flat command rail sits above the
canvas and one slim count/legend rail below it; the breadcrumb trail *is*
the title block, ending in the current page's name as the heading, and graph
controls form a joined icon ribbon with visible tooltips and accessible
names. Fit all leads the ribbon, followed by zoom, selected-path motion, lane
reset, and interaction help. Nodes are surface-1 cards with a 1px border and
an 8–10px radius; the selected node gains a 2px Azure border on the soft blue
fill, and keyboard focus reads as a ring around the node. Connectors are
semantic (network green, structure blue, data violet, identity orange-red,
monitoring amber), 1.5px idle and 2.5px when active, and unrelated nodes and
edges recede while a relationship is inspected.

The initial camera shows a readable connected core rather than
indiscriminately fitting every secondary region. Estate entry also includes
collapsed subscription orientation bars; group entry includes connected
resources and external context but excludes the unconnected shelf;
neighbourhood entry includes the subject and its one-hop peers. Each scope
settles inward from a slightly wider opening frame. Fit all is always
available. Camera bounds adapt to graph level, target density, and viewport
size so sparse maps use the canvas instead of remaining artificially capped
below 1:1. Group and resource drill actions use Cytoscape's native viewport
animation for spatial continuity; reduced-motion mode applies the destination
camera immediately.

Estate layouts use responsive subscription grids plus compact collapsed lanes.
Group layouts prioritise contained and connected resources, place external
neighbours on boundary rails, and keep resources without drawn relationships
in a lower secondary region. Neighbourhood layouts centre the selected resource
with inbound relationships left, outbound right, and second hops in stable
outer columns. The group field labels its external context, connected-service
rail, and unconnected shelf, while connection-aware ordering keeps related
services aligned with the network core. Semantic zoom removes secondary copy
before primary labels and finally uses icon/count overview tiles; it never
scales text below 11px.

Keyboard focus, pointer hover, and the selected neighbourhood subject trace an
incident route in that priority order. Active connectors use deterministic
orthogonal channels and boundary anchor slots; unrelated graph content recedes
without disappearing. Every logical link owns an invisible source and target
port on the real node or compound-frame boundary. Ports on the same side are
ordered by the opposite endpoint's spatial position, clamped clear of corners,
and separated before Cytoscape draws the round-taxi route. This is what keeps
shared endpoints from collapsing into one trunk; direct logical-node edges are
not an acceptable fallback.

The paint stack is field → idle links → active links → relationship labels →
node labels and controls. Relationship text belongs to the active trace and is
a DOM overlay above every connector rather than Cytoscape edge paint. Opaque
token-backed plates sit beside the remote endpoint, use stable slots when
several links share it, avoid node and label obstacles, and resynchronise
through pan, zoom, drag, resize, and camera animation. Synthetic port ids never
participate in trace, counts, camera targets, navigation, or accessible
summaries. The independently announced route summary remains available to
assistive technology. Hidden windows, the pause control, and reduced-motion
preferences stop connector motion. The complete engineering and regression
contract is [Desktop map](../development/desktop-relationships.md).

One click and Enter share the same primary activation model: a group opens its
group map and a resource opens its resource record. A separate, visible
relationship-count action opens that resource's neighbourhood, with **R** as
its roving-keyboard shortcut. The resource record repeats that action as
**Explore N relationships**. Temporal Back restores the exact preceding record
or relationship scope; hierarchical breadcrumbs provide direct estate and
group exits. Aggregate tiles expand in place to list their members; there is no
topology drawer or double-click navigation. Graph labels form one roving Tab
stop with spatial arrow-key movement. The bottom rail says "N relationships
drawn as N connectors", because those are distinct measures and the sentence
says how they relate, and contains the relationship-kind legend (collapsed
behind a labelled control at compact widths). Failed refreshes retain and mark
the last successful graph as stale, with Retry and Revert actions and an
`aria-live` announcement.

## Settings layout

Settings uses the same tokens: a softly bounded tenant list beside a matching
editor card. Top-level areas are Tenants, Shared defaults and Application,
grouped into a compact tab control. Selected tabs and tenants use the
`--az-selected` fill; inputs use surface-1 so they stay distinct from the
canvas without becoming heavy in dark mode. Tenant Connection, Collection &
audit and Report branding are separate panels; selecting a tenant here changes
the editor, while the command bar's selector changes the active estate.
Identity and credentials share columns when the tenant editor has at least
720px of width and stack below that. At a window width of 760px or less, the
tenant list moves above the editor. Advanced typography and document geometry
stay expandable. Fields carry explicit labels and inherited or overridden
states; validation identifies invalid fields. All control text remains at
least 11px.

The form owns vertical scrolling inside the app workspace. Save/Discard lives
outside that scroll container so neither viewport height nor long branding
forms can hide it. Changing area, panel or tenant resets the form to its top.
Connection test progress and results use a native `dialog` with the shared
modal treatment. Application's System, Light and Dark labels are centred in
equal-width buttons within a compact, content-sized control; the selected
choice uses the quiet selected fill and exposes its pressed state.

The implementation is in
[`SettingsView.tsx`](../../desktop/src/components/SettingsView.tsx) and
[`SettingsConnectionDialog.tsx`](../../desktop/src/components/SettingsConnectionDialog.tsx).

## Accessibility

The target is WCAG 2.2 AA: body text at or above 4.5:1, large text at or
above 3:1, a visible focus state on every control, findings never conveyed
by colour alone, a text or tooltip equivalent for every chart, labelled
icon-only actions and targets of at least 32×32px. The one knowing
exception is the primary button: the specified Azure blue under a white
13px label measures about 4.3:1, marginally under AA; darken
`--az-primary-fill` to the hover shade if that ever needs to be strict.
