# Desktop design language — "Field Report"

The desktop app wears the printed estate report made interactive: IBM Plex
Serif carries identity, Plex Sans does the work, Plex Mono marks
machine-shaped evidence. Paper surfaces with hairline rules in light; dark is
a warm-charcoal **1:1 token remap** ("night reading"), never a separate
design. The tokens live in
[`desktop/src/styles.css`](../../desktop/src/styles.css); a rendered version
of this sheet is at [design.html](design.html).

Three rules are non-negotiable, straight from review:

- **No numbering chrome.** Figures end with a short italic caption, never
  "Fig. 3.1". Headings are names, not chapter numbers.
- **Selection is a quiet evidence-tint fill** (`--evidence`) with ink text and
  weight — never a coloured bar or left-edge accent.
- **Colour is reserved** for data (the category set) and signals (the
  severity set). Chrome stays paper-and-ink.

## Modes

Theme is a tri-state in Settings: **System** (default — follows the OS via
`prefers-color-scheme`), Light, Dark. An explicit choice pins
`data-theme="light|dark"` on `<html>` and persists in `localStorage`
(`azdocs-theme`); System removes the attribute. Every light token has exactly
one dark counterpart — components never branch on the mode.

## Tokens

| Token | Role | Light | Dark |
|---|---|---|---|
| `--paper` | app ground | `#faf8f4` | `#14181d` |
| `--surface` | raised panels, grids | `#ffffff` | `#1b2129` |
| `--evidence` | JSON/mono blocks, selection fill | `#f3efe7` | `#10141a` |
| `--ink` | headings, rules, primary button | `#1c2430` | `#e9e4da` |
| `--body` | running text | `#3a4450` | `#c7c0b4` |
| `--muted` | secondary text | `#5a6470` | `#9a958a` |
| `--faint` | labels, captions | `#7a828c` | `#7d786e` |
| `--faintest` | placeholders, counts | `#9aa2ac` | `#635f57` |
| `--line-strong` | panel borders, column splits | `#d8d2c6` | `#3a4048` |
| `--line` | row separators | `#eae6de` | `#262c34` |
| `--accent` | links, active tabs, single-hue charts | `#0b5da8` | `#5aa7e8` |
| `--coral` | high severity, risk | `#a83a22` | `#e07a5f` |
| `--amber` | medium severity, warnings | `#8a6d00` | `#c9a83a` |
| `--green` | healthy, resolved, complete | `#13734b` | `#63bd8f` |

### Category / chart set

Fixed order, never cycled — the top slots of the app's Rust
`category_color()` set. Charts fold the tail into *Other*; type dots
elsewhere may use the full set. Both mode palettes are CVD-validated against
their surfaces; only the storage slot changes in dark.

| Token | Category | Light | Dark |
|---|---|---|---|
| `--cat-compute` | compute | `#0078d4` | `#0078d4` |
| `--cat-networking` | networking | `#107c10` | `#107c10` |
| `--cat-storage` | storage | `#c19c00` | `#a98a00` |
| `--cat-databases` | databases | `#b146c2` | `#b146c2` |
| `--cat-appservices` | app services | `#d83b01` | `#d83b01` |
| `--cat-other` | everything else | `#5a6470` | `#5a6470` |

Relationship kind classes (`--kind-network` …) map onto the same set. The
Cytoscape stage reads `--graph-*` tokens at build time and rebuilds when the
resolved theme changes, so both modes paint from one palette.

### Severity

Status colours are reserved and always paired with the word — never colour
alone: high = coral, medium = amber, low = muted, info = accent. Rows
referenced by open findings carry a dagger (†) in every table.

## Type

IBM Plex throughout — the same family the PDF vendors, so app and report are
one product family. Serif Regular/SemiBold is bundled with the frontend
(`desktop/src/assets/fonts/`, OFL); Sans and Mono are served from the shared
`data/fonts/` directory.

| Level | Face | Size | Use |
|---|---|---|---|
| Display | Plex Serif 600 | 26–30px | view titles, hero numerals |
| Chapter | Plex Serif 600 | 16–20px | section and panel headings, table heads |
| Body | Plex Sans 400 | 13–14px | descriptions and supporting copy |
| Table | Plex Sans 400/600 | 12.5px | ledger rows, inspector detail |
| Label | Plex Sans 600 caps | 11px | column labels, metadata, tracking `0.1em` |
| Evidence | Plex Mono 400 | 11–12px | ARM ids, tags, deltas, query names |

**Nothing below 11px** — the old 6–10px floor is retired.

## Structure

- **2px ink rule** anchors mastheads, stat strips and footers; **1px strong
  hairline** for panel edges and column splits; **1px hairline** for rows.
  Tone and rules build hierarchy before shadow does.
- Spacing rhythm `4 · 8 · 12 · 20 · 32`; radii `3px` (controls and tag chips),
  `6px` (panels), and `999px` for the deliberately rounded summary capsule.
  The rest of the interface stays near-square and print-like.
- The **stat strip** remains the summary idiom when metrics are primary page
  content: serif numerals between rules, one cell per measure, coral only when
  the measure is a risk. Use **summary capsules** instead when two to six
  secondary orientation facts can occupy spare header or toolbar space without
  competing with the title. The group wraps as a unit before it squeezes
  identity or action text.
- Side navigation is a slim IDE-style list (~176px): icon + label rows,
  tree subtrees with guide lines, Settings pinned to the bottom, a mono coral
  count as the Findings badge.
- Estate disclosure and scope selection are independent. Subscriptions open to
  resource groups and groups open to resource rows without changing the active
  scope. Azure entities use the vendored Azure artwork throughout; generic
  interface glyphs are reserved for controls, navigation, and signals.
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

### Relationships workspace

Relationships alone is full-bleed. It uses one flat command rail above the
canvas and one slim count/legend rail below it; breadcrumbs are part of the
title block and secondary graph controls live in a labelled More menu. The
initial camera shows a readable connected core rather than indiscriminately
fitting every secondary region. Fit all is always available. Camera bounds
adapt to graph level, target density, and viewport size so sparse maps use the
canvas instead of remaining artificially capped below 1:1. Group and resource
drill actions use Cytoscape's native viewport animation for spatial continuity;
reduced-motion mode applies the destination camera immediately.

Estate layouts use responsive subscription grids plus compact collapsed lanes.
Group layouts prioritise contained and connected resources, place external
neighbours on boundary rails, and keep resources without drawn relationships
in a lower secondary region. Neighbourhood layouts centre the selected resource
with inbound relationships left, outbound right, and second hops in stable
outer columns. Semantic zoom removes secondary copy before primary labels and
finally uses icon/count overview tiles; it never scales text below 11px. Edge
labels belong only to the selected path.

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
bottom rail says **source relationships**
and **connectors**, because those are distinct measures, and contains the
relationship-kind legend (collapsed behind a labelled control at compact
widths). Failed refreshes retain and mark the last successful graph as stale,
with Retry and Revert actions and an `aria-live` announcement.

## Components

- **Primary button**: ink block, paper text — inverts with the mode.
- **Quiet button**: surface fill, strong hairline border.
- **Active tab**: 2px accent underline (tabs are the one place the accent
  underlines; navigation never does).
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
- **Charts**: horizontal bars direct-labelled with the value; one hue for one
  measure, category colours only for categorical identity; captions are one
  short italic line.
