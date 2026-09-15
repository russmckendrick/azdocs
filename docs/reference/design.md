# Desktop design language

The desktop combines a balanced Overview dashboard with focused evidence
workspaces. IBM Plex Serif carries identity, Plex Sans does the work, and Plex
Mono marks machine-shaped evidence. Paper surfaces and hairline rules frame
the content over a soft cool-grey-to-Azure atmosphere in light mode; dark is a
charcoal-to-deep-Azure **1:1 token remap** of the same composition. This palette
belongs to the desktop app only; PDF and DOCX continue to use the selected
data-driven document theme. The tokens live in
[`desktop/src/styles.css`](../../desktop/src/styles.css) — the only place a
colour value is written down. [design.html](design.html) shows them as
swatches, alongside the type scale and the brand marks; it is generated from
those sources by
[`tools/build_design_sheet.py`](tools/build_design_sheet.py), so never edit it
by hand.

Three rules are non-negotiable, straight from review:

- **No numbering chrome.** Figures end with a short italic caption, never
  "Fig. 3.1". Headings are names, not chapter numbers.
- **Selection is a quiet evidence-tint fill** (`--evidence`) with ink text and
  weight — never a coloured bar or left-edge accent.
- **Ambient colour stays soft.** Cool grey/Azure washes can carry large chrome
  and surface regions. Saturated colour is reserved for data (the category
  set), signals (the severity set), focus and primary actions.

## Modes

Theme is a tri-state in Settings: **System** (default — follows the OS via
`prefers-color-scheme`), Light, Dark. An explicit choice pins
`data-theme="light|dark"` on `<html>` and persists in `localStorage`
(`azdocs-theme`); System removes the attribute. Every light token has exactly
one dark counterpart — components never branch on the mode.

## Tokens

Values belong to `styles.css`; the generated [design sheet](design.html)
shows the light and dark swatches without maintaining another palette here.

| Token | Role |
|---|---|
| `--paper` | app ground |
| `--surface` | raised panels, grids |
| `--evidence` | JSON/mono blocks, selection fill |
| `--ink` | headings, high-contrast anchors, primary button |
| `--body` | running text |
| `--muted` | secondary text |
| `--faint` | labels, captions |
| `--faintest` | placeholders, counts |
| `--line-strong` | panel borders, column splits |
| `--line` | row separators |
| `--accent` | links, active tabs, single-hue charts |
| `--coral` | high severity, risk |
| `--amber` | medium severity, warnings |
| `--green` | healthy, resolved, complete |

The solid tokens remain available wherever CSS or Cytoscape requires a colour.
Four companion atmosphere tokens carry the soft gradients: `--ground-wash`
is shared by the app canvas, masthead and status rail, with navigation left
transparent over that canvas; `--surface-wash` serves controls and panels, and
`--evidence-wash` selected evidence. `--action-wash` is the higher-contrast
Azure gradient reserved for the primary action.

### Category / chart set

Fixed order, never cycled — the top slots of the app's Rust
`category_color()` set. Charts fold the tail into *Other*; type dots
elsewhere may use the full set. Check both mode palettes against their surfaces
and with colour-vision deficiency simulation when changing them; labels must
retain the meaning without colour. Only the storage slot changes in dark.

| Token | Category |
|---|---|
| `--cat-compute` | compute |
| `--cat-networking` | networking |
| `--cat-storage` | storage |
| `--cat-databases` | databases |
| `--cat-appservices` | app services |
| `--cat-other` | everything else |

Relationship kind classes (`--kind-network` …) map onto the same set. The
Cytoscape stage reads `--graph-*` tokens at build time and rebuilds when the
resolved theme changes, so both modes paint from one palette.

### Severity

Status colours are reserved and always paired with the word — never colour
alone: high = coral, medium = amber, low = muted, info = accent. Rows
referenced by open findings carry a dagger (†) in every table.

## Type

The desktop uses IBM Plex throughout. PDF bundles Plex as its fallback; Field
Report prefers installed Charter, Arial and Courier New for print, as described
in [Fonts](themes.md#fonts). Serif Regular/SemiBold is bundled with the frontend
(`desktop/src/assets/fonts/`, OFL); Sans and Mono are served from the shared
`data/fonts/` directory.

| Level | Face | Size | Use |
|---|---|---|---|
| Display | Plex Serif 600 | 26–30px | view titles, hero numerals |
| Overview KPI | Plex Serif 600 | 36px | four primary dashboard values, tabular numerals |
| Chapter | Plex Serif 600 | 16–20px | section and panel headings, table heads |
| Body | Plex Sans 400 | 13–14px | descriptions and supporting copy |
| Table | Plex Sans 400/600 | 12.5px | ledger rows, inspector detail |
| Label | Plex Sans 600 caps | 11px | column labels, metadata, tracking `0.1em` |
| Evidence | Plex Mono 400 | 11–12px | ARM ids, tags, deltas, query names |

**Nothing below 11px** — the old 6–10px floor is retired.

## Structure

- **2px ink rules** are reserved for stat strips and other high-value summary
  anchors. The masthead, status rail and record headers use a **1px strong
  hairline**; quieter section boundaries and column splits use a **1px
  hairline**. Tone and spacing build hierarchy before rules or shadow do.
- Spacing rhythm `4 · 8 · 12 · 20 · 32`; radii `3px` (controls and tag chips),
  `6px` (panels), and `999px` for the deliberately rounded summary capsule.
  Overview uses `12px` KPI and chart panels and a `14px` detail dialog; these
  are a local dashboard treatment, not a replacement for evidence workspaces.
- The **stat strip** remains the summary idiom outside Overview when metrics
  are primary page content: serif numerals between rules, one cell per measure,
  coral only when the measure is a risk. **Overview is the user-approved
  exception**, with four KPI cards and chart panels described below.
  Use **summary capsules** when two to six
  secondary orientation facts can occupy spare header or toolbar space without
  competing with the title. The group wraps as a unit before it squeezes
  identity or action text.
- The masthead identity is the azdocs logo, with no tagline. Snapshot, search
  and collection controls retain their own roles alongside it.
- Side navigation is a slim list (~176px) of icon + label rows, with Settings
  pinned to the bottom and a mono coral count for high-severity findings.
  Minimise collapses it to a 68px icon rail; the preference persists across
  sessions. Both widths retain accessible names, native title tooltips and
  `aria-current`; the toggle names its action and exposes its expanded state.
  Use the vendored Azure SVGs in navigation and dashboard metrics: Compliance
  for findings and Resource Graph Explorer for Map and relationships.
- Estate disclosure and scope selection are independent. Subscriptions open to
  resource groups and groups open to resource rows without changing the active
  scope. Azure entities use the vendored Azure artwork throughout; generic
  interface glyphs identify controls and actions. Preserve the supplied artwork
  rather than recolouring it as decorative chrome.
- **One working surface per route.** Estate keeps its hierarchy rail because
  scope is part of browsing. A resource opens as a dedicated evidence sheet:
  all stored properties, SKU, identity, tags, findings, edge properties, ARM
  identifiers, and provenance are rendered without tabs or truncation. A
  shape-driven evidence renderer presents scalar and nested fields as compact
  property rows, and collections as real data tables rather than raw JSON or
  an embedded object inspector. Resource context is part of that linear
  evidence flow, never a fixed side column that taxes the width of long tables.
  Location, kind, finding, relationship, and tag summaries occupy compact
  header capsules instead of a second full-width stat band.
  Finding evidence remains an on-demand drawer. Relationships use direct
  navigation from groups to group maps and from resources to their records;
  a resource card's labelled relationship-count action opens its neighbourhood.
  Back restores the preceding record or graph scope, while breadcrumbs navigate
  the estate/group/resource hierarchy. Overview
  scrolls as one document rather than splitting into independently scrolling
  columns. History gives the comparison ledger more room than the snapshot
  selector, and long query-run evidence stays collapsed until requested.
- Large collections must not make navigation grow without bound. Resource
  types and inventory queries use labelled selectors; lists and tables own
  their scroll area, while detail never permanently narrows them.

### Overview dashboard

Overview provides orientation in one scrolling document. Four equally weighted
KPI cards lead: resources, audit findings, any-tag coverage, and stored
relationships. Each combines a Serif value, Azure icon and short context line;
only a findings value with high-severity results uses the risk colour.

The responsive twelve-column chart grid contains resource history, findings
by severity, resource types, regions, tag coverage by subscription, collection
coverage, changes since the previous snapshot, and priority checks. A wide
history panel sits beside the severity donut; composition panels follow, then
collection coverage and changes, with priority checks across the full width.
Container queries adapt to available workspace width: below 950px, middle
panels rebalance; below 650px, the KPI cards form two columns and chart panels
stack. Labels stay at least 11px, with direct values and full-name tooltips
where a chart label is ellipsised.

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
native `dialog`. Its heading names the selection and scope, the body shows
matching stored records in progressive batches, and the footer links to the
exact result filters or selected snapshot. Resource and relationship rows
provide direct record or neighbourhood navigation. The dialog has a labelled
close button, native focus containment, Escape and backdrop dismissal, and a
scrolling body that leaves the header and actions accessible. Result pages
show the originating selection, a return action and a clear-filter control.

Motion helps explain state changes: KPI numbers interpolate from their current
value, bars and donut segments ease to their new values, the history line draws
in, and the dialog and backdrop enter together. Sidebar width changes immediately;
labels fade and slide in on expansion without animating the workspace layout.
Reduced motion removes these animations and applies numeric changes directly,
including when the preference changes during an animation.

The implementation is in
[`OverviewView.tsx`](../../desktop/src/components/OverviewView.tsx),
[`DashboardModal.tsx`](../../desktop/src/components/DashboardModal.tsx),
[`dashboard-model.ts`](../../desktop/src/components/dashboard-model.ts) and
[`dashboard-motion.ts`](../../desktop/src/components/dashboard-motion.ts).
Result navigation follows
[`navigation-state.ts`](../../desktop/src/navigation-state.ts); all wording
continues to come from the typed [labels](labels.md).

### Inventory evidence disclosures

Inventory keeps operational summaries and each query's recorded provenance in
native disclosures, collapsed until requested. They use the existing paper,
ink and hairline treatment. Rust owns the summary analysis and labelled
wording; the frontend renders the supplied statuses, notes, columns and cells.
Evidence semantics, source limits and historical provenance are documented in
[Operational and access evidence](operational-evidence.md).

Expanded disclosures contain bounded scrolling panels (320px maximum height).
Long source URLs, query hashes and saved KQL wrap within the available width;
KQL retains its line breaks and Mono face. The query record is keyboard
focusable and scrolls, and the outer workspace also scrolls when disclosures
exceed a short window. Preserve the raw table's minimum viewport (160px) so
opening provenance cannot collapse the ledger or make its footer unreachable.
These behaviours live in
[`InventoryView.tsx`](../../desktop/src/components/InventoryView.tsx) and the
inventory rules in [`styles.css`](../../desktop/src/styles.css).

### Map workspace

Map alone is full-bleed. It uses one flat command rail above the
canvas and one slim count/legend rail below it; the breadcrumb trail *is*
the title block, ending in the current page's name as the heading, and graph controls form a joined icon ribbon with visible tooltips
and accessible names. Fit all leads the ribbon, followed by zoom, selected-path
motion, lane reset, and interaction help. The
initial camera shows a readable connected core rather than indiscriminately
fitting every secondary region. Estate entry also includes collapsed
subscription orientation bars; group entry includes connected resources and
external context but excludes the unconnected shelf; neighbourhood entry
includes the subject and its one-hop peers. Each scope settles inward from a
slightly wider opening frame. Fit all is always available. Camera bounds
adapt to graph level, target density, and viewport size so sparse maps use the
canvas instead of remaining artificially capped below 1:1. Group and resource
drill actions use Cytoscape's native viewport animation for spatial continuity;
reduced-motion mode applies the destination camera immediately.

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
topology drawer or double-click navigation.
Graph labels form one roving Tab stop with spatial arrow-key movement.
Selection uses `--evidence`; the accent is reserved for keyboard focus. The
bottom rail says "N relationships drawn as N connectors", because those are
distinct measures and the sentence says how they relate, and contains the
relationship-kind legend (collapsed behind a labelled control at compact
widths). Failed refreshes retain and mark the last successful graph as stale,
with Retry and Revert actions and an `aria-live` announcement.

## Components

- **Primary button**: deep-grey-to-Azure gradient with stable light text in
  both modes.
- **Quiet button**: soft surface wash, strong hairline border.
- **Active tab**: 2px accent underline for evidence tabs. Settings area and
  panel navigation uses the quiet evidence fill, as described below.
- **Tag chip**: mono on evidence fill; a **missing** required tag is a dashed
  coral outline.
- **Summary capsule**: a short, read-only label/value pair for orientation
  facts such as location, kind, scope, status, or count. It uses a transparent
  paper surface, one strong hairline, and a fully rounded silhouette (`999px`):
  11px tracked Sans for the label and 13.5px semibold Serif for the value.
  Keep two to six together in a header or toolbar; colour only a signal value,
  never the capsule chrome. Do not use capsules for actions, filters, long IDs,
  primary dashboard metrics, or as a substitute for tables and ledgers.
- **Evidence blocks** (stored JSON, ARM ids): mono on `--evidence` with a
  hairline border, never a dark backplate in light mode.
- **Charts**: bars are direct-labelled with the value; one hue for one measure,
  category colours only for categorical identity. Overview adds a history line
  and a severity donut with named, counted legend controls. Captions and scope
  notes explain the evidence without numbering.

## Settings layout

Settings uses the Field Report tokens and a restrained, softly bounded tenant
list beside a matching editor surface. Top-level areas are Tenants, Shared
defaults and Application, grouped into a compact tab control rather than a
full-width page rule. Selected tabs and tenants use the evidence wash and a
hairline boundary; inputs use the solid surface token so they stay distinct
from the pale light-mode canvas without becoming heavy in dark mode.
Tenant Connection, Collection & audit and Report branding are separate panels;
selecting a tenant here changes the editor, while the masthead selector changes
the active estate. Identity and credentials share columns when the tenant
editor has at least 720px of width and stack below that. At a window width of
760px or less, the tenant list moves above the editor. Advanced typography and
document geometry stay expandable. Fields carry explicit labels and inherited
or overridden states; validation identifies invalid fields. All control text
remains at least 11px.

The form owns vertical scrolling inside the app workspace. Save/Discard lives
outside that scroll container so neither viewport height nor long branding
forms can hide it. Changing area, panel or tenant resets the form to its top.
Selection uses the quiet evidence fill. At smaller desktop widths, fields
reflow and masthead tenant and snapshot selectors shrink to preserve actions.

Connection test progress and results use a native `dialog` with the shared
dashboard modal treatment. The heading names the test and tenant; the body
scrolls while the heading and close controls remain accessible. Native focus
containment and Escape dismissal keep interaction in the dialog, and closing
returns focus to the test control when it is enabled. The form retains a short
status and a link to reopen the results. Application's System, Light and Dark
labels are centred in equal-width buttons within a compact, content-sized
control; the selected choice uses the quiet evidence fill and exposes its
pressed state.

The implementation is in
[`SettingsView.tsx`](../../desktop/src/components/SettingsView.tsx) and
[`SettingsConnectionDialog.tsx`](../../desktop/src/components/SettingsConnectionDialog.tsx).
