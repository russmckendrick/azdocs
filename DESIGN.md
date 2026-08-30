---
name: azdocs Desktop
description: The printed Azure estate report made interactive through the Field Report design language.
colors:
  paper: "#faf8f4"
  surface: "#ffffff"
  evidence: "#f3efe7"
  ink: "#1c2430"
  body: "#3a4450"
  muted: "#5a6470"
  faint: "#7a828c"
  faintest: "#9aa2ac"
  hairline-strong: "#d8d2c6"
  hairline: "#eae6de"
  accent: "#0b5da8"
  signal-coral: "#a83a22"
  signal-amber: "#8a6d00"
  signal-green: "#13734b"
  compute-blue: "#0078d4"
  network-green: "#107c10"
  storage-gold: "#c19c00"
  database-purple: "#b146c2"
  app-orange: "#d83b01"
  night-paper: "#14181d"
  night-surface: "#1b2129"
  night-evidence: "#10141a"
  night-ink: "#e9e4da"
  night-body: "#c7c0b4"
  night-muted: "#9a958a"
  night-faint: "#7d786e"
  night-faintest: "#635f57"
  night-hairline-strong: "#3a4048"
  night-hairline: "#262c34"
  night-accent: "#5aa7e8"
  night-signal-coral: "#e07a5f"
  night-signal-amber: "#c9a83a"
  night-signal-green: "#63bd8f"
typography:
  display:
    fontFamily: "Plex Serif, Georgia, serif"
    fontSize: "26px"
    fontWeight: 600
    lineHeight: 1.15
    letterSpacing: "-0.01em"
  chapter:
    fontFamily: "Plex Serif, Georgia, serif"
    fontSize: "17px"
    fontWeight: 600
    lineHeight: 1.25
  body:
    fontFamily: "Plex, Segoe UI, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Plex, Segoe UI, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "0.1em"
  evidence:
    fontFamily: "Plex Mono, ui-monospace, monospace"
    fontSize: "11.5px"
    fontWeight: 400
    lineHeight: 1.55
rounded:
  control: "3px"
  graph-card: "4px"
  panel: "6px"
  capsule: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "20px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.control}"
    padding: "7px 14px"
  button-quiet:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.body}"
    rounded: "{rounded.control}"
    padding: "6px 12px"
  panel-evidence:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.panel}"
  summary-capsule:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.capsule}"
    padding: "5px 8px 6px"
  relationship-label:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.muted}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "4px 7px"
---

# Design System: azdocs Desktop

## Overview

**Creative North Star: "The Field Report"**

azdocs is the printed Azure estate report made interactive. IBM Plex Serif
carries identity, Plex Sans does the work, and Plex Mono marks machine-shaped
evidence. Light mode is paper, ink, and hairline rules; dark mode is the same
composition remapped to warm charcoal for night reading, never a separate
visual world.

The interface is calm around stored facts and vivid only where relationship
kind, risk, focus, or state needs attention. It behaves like a professional
desktop instrument without adopting generic observability-dashboard chrome.
The authoritative detailed sheet is
[docs/reference/design.md](docs/reference/design.md), and the live tokens are
implemented in [desktop/src/styles.css](desktop/src/styles.css).

**Key Characteristics:**

- Editorial structure with desktop-dense evidence.
- One light-first token system with an exact dark remap.
- Native Azure service artwork on transparent icon surfaces.
- Colour reserved for relationship data and semantic signals.
- Deterministic cartographic relationship maps with legible routes and labels.

## Colors

The palette is paper-and-ink first. Azure blue is the interaction accent;
relationship families reuse the fixed category set; coral, amber, and green
are signals rather than decoration. Every status colour is paired with text.

**The Reserved Colour Rule.** Chrome stays neutral. Use category colours for
data identity and severity colours for explicit signals; do not tint ordinary
containers or navigation for atmosphere.

**The One Night Map Rule.** Dark mode remaps the same semantic tokens and
preserves the same hierarchy, density, and geometry. Components never branch
into a second dark-mode design.

## Typography

**Display Font:** IBM Plex Serif (with Georgia fallback)

**Body Font:** IBM Plex Sans (with Segoe UI and system fallbacks)

**Label/Mono Font:** IBM Plex Mono (with the platform monospace fallback)

Serif names the report and its major evidence sections; Sans carries controls,
resource names, labels, and explanation; Mono is limited to values whose shape
is part of the evidence, such as ARM ids, tags, query names, and deltas.

### Hierarchy

- **Display** (600, 26–30px): view titles and hero numerals.
- **Chapter** (600, 16–20px): panel and section headings.
- **Body** (400, 13–14px): descriptions, supporting copy, and controls.
- **Table** (400/600, 12.5px): ledger rows and inspector evidence.
- **Label** (600, 11px, 0.1em): tracked metadata and compact region labels.
- **Evidence** (400, 11–12px): machine-shaped values only.

**The Eleven Pixel Floor Rule.** Nothing is rendered below `11px`. Semantic
zoom removes secondary content before primary text and never solves density by
making labels unreadable.

## Layout

A masthead sits above a slim IDE-style side navigation and one primary working
surface, with a status rail below. The Estate route alone keeps a hierarchy
rail. Resource evidence is one full-width, scrollable sheet rather than tabs
or a permanently narrowing inspector column. Short supporting facts may use a
compact capsule group; evidence tables and primary metrics may not.

Relationships is the sole full-bleed workspace. A flat command rail sits above
the cartographic field and a slim count/legend rail below. Estate, group, and
neighbourhood scopes have different deterministic compositions and different
readable entry sets. The opening camera starts wider than the final fit and
settles inward; **Fit all** is explicit. The full routing, layering, camera, and
anti-regression contract is
[docs/development/desktop-relationships.md](docs/development/desktop-relationships.md).

**The Logical Graph Rule.** Rust owns topology truth. Frontend layout may add
synthetic connector ports, but it must not change graph semantics, honest
counts, filtering, or the typed DTO.

**The Route Before Grid Rule.** External and service rails are ordered by
their connected peers before name. A tidy alphabetical grid is not acceptable
when it causes long crossings or routes through sibling cards.

## Elevation & Depth

The system is flat by default. Paper tone, evidence tone, strong hairlines,
and whitespace establish hierarchy before shadow. The neutral relationship
field adds a fine grid and restrained centre vignette; it does not become a
glowing or decorative backdrop. Shadows are reserved for genuinely lifted
overlays and remain quiet.

**The Tonal-First Rule.** If a boundary or surface change can express the
hierarchy, use it before adding shadow, blur, or glow.

## Shapes

Controls are gently squared (`3px`), graph cards use `4px`, and panels use
`6px`. Compound graph frames use dashed near-square boundaries. Fully rounded
shapes are reserved for short read-only summary capsules and tiny count pills;
they are not the default component silhouette.

Graph connectors are orthogonal round-taxi paths anchored to dedicated points
on node boundaries. Corner radii soften turns without turning the map into a
free-form node-link diagram.

## Components

### Buttons

- **Primary:** ink block with paper text; one principal command per toolbar.
- **Quiet:** surface fill, strong hairline, and body text.
- **Focus:** a visible accent outline; colour reinforces rather than replaces
  focus semantics.

### Cards and evidence containers

- Resource and group cards use a surface fill and one strong hairline.
- Selection uses the quiet evidence fill with ink text, never a coloured side
  bar.
- External, aggregate, and collapsed-subscription cards retain distinct dashed
  treatments.

### Summary capsules

Capsules orient; they do not operate. Use two to six short label/value facts in
spare header or toolbar space. Never use them for actions, filters, long ids,
primary metrics, or as a substitute for a table.

### Relationship map

- A logical relationship renders through its own invisible source and target
  boundary ports. Shared endpoints fan into spatially ordered taxi channels.
- Paint order is field → idle links → active links → relationship labels →
  node labels and controls.
- Relationship text is an opaque DOM plate beside the remote endpoint, never
  a Cytoscape edge label.
- Keyboard focus outranks pointer hover, which outranks the selected subject
  when choosing the traced route.
- Motion is directional and restrained; it stops while paused, hidden, or
  reduced motion is active.

**The Labels Above Lines Rule.** No connector geometry may paint in a DOM
label layer, and no relationship label may be painted on the Cytoscape canvas.
This rule holds throughout pan, zoom, drag, resize, and camera animation.

## Do's and Don'ts

### Do

- **Do** use the supplied Microsoft Azure icon for every known service type.
- **Do** keep graph ordering, port assignment, camera targets, and output
  deterministic for the same stored snapshot.
- **Do** preserve the equation `drawn + folded + aggregated == total` and count
  everything hidden by a filter.
- **Do** give every rendered link its own source and target boundary ports,
  ordered by the opposite endpoint's spatial position.
- **Do** keep relationship plates opaque, collision-aware, and above every
  connector for the full interaction lifecycle.
- **Do** keep click and Enter equivalent, spatial keyboard navigation intact,
  and accessible route summaries independent of the canvas.

### Don't

- **Don't** draw direct centre-to-centre or default-boundary logical-node
  edges, reuse one port for unrelated links, or collapse routes into a shared
  trunk.
- **Don't** replace connection-aware rails with a name-only grid or route
  through sibling cards.
- **Don't** render relationship text with Cytoscape or use synthetic port ids
  for tracing, counts, camera fits, navigation, or accessibility.
- **Don't** include the unconnected shelf or second-hop context in the opening
  camera; **Fit all** owns that job.
- **Don't** hardcode graph colours, shrink text below `11px`, create a separate
  dark composition, or add decorative colour to neutral chrome.
- **Don't** let the webview fetch Azure, open SQLite, access credentials, or
  silently cap graph content.
