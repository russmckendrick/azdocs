---
name: azdocs Desktop
description: The printed estate report made interactive — the "Field Report" design language.
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
  coral: "#a83a22"
  amber: "#8a6d00"
  green: "#13734b"
darkColors:
  paper: "#14181d"
  surface: "#1b2129"
  evidence: "#10141a"
  ink: "#e9e4da"
  body: "#c7c0b4"
  muted: "#9a958a"
  faint: "#7d786e"
  faintest: "#635f57"
  hairline-strong: "#3a4048"
  hairline: "#262c34"
  accent: "#5aa7e8"
  coral: "#e07a5f"
  amber: "#c9a83a"
  green: "#63bd8f"
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
    border: "1px solid {colors.hairline-strong}"
    rounded: "{rounded.control}"
    padding: "6px 12px"
  panel-evidence:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    border: "1px solid {colors.hairline-strong}"
    rounded: "{rounded.panel}"
  summary-capsule:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.capsule}"
    padding: "5px 8px 6px"
---

# Design System: azdocs Desktop

**Creative North Star: "The Field Report"** — the printed estate report made
interactive. The app, the PDF and the DOCX are one product family: IBM Plex
Serif carries identity, Plex Sans does the work, Plex Mono marks
machine-shaped evidence. Paper surfaces and hairline rules in light; dark is a
warm-charcoal 1:1 token remap ("night reading"), never a separate design.

The authoritative, full version of this sheet lives at
[docs/reference/design.md](docs/reference/design.md) (rendered:
[docs/reference/design.html](docs/reference/design.html)); the tokens are
implemented once in [desktop/src/styles.css](desktop/src/styles.css).

## The rules that shape every screen

- **No numbering chrome.** Headings are names; figures end with a short
  italic caption, never "Fig. 3.1".
- **Selection is a quiet evidence-tint fill** with ink text and weight —
  never a coloured bar or accent edge.
- **Colour is reserved.** Data wears the category set (the app's own Rust
  `category_color()` slots, CVD-validated per surface); signals wear the
  severity set, always paired with the word. Chrome stays paper-and-ink.
- **Rules over boxes, tone over shadow.** A 2px ink rule anchors mastheads
  and stat strips; hairlines separate everything else. Controls and panels
  remain near-square at 3px/6px. Short, read-only summary capsules are the one
  deliberately fully rounded exception.
- **Nothing below 11px.** Density comes from rhythm, not from shrinking type.
- **Summary capsules orient; they do not operate.** Use a compact capsule for a
  short label/value fact such as location, kind, count, scope, or status when
  two to six supporting facts can share spare header or toolbar space. Keep
  the label in 11px tracked Sans, the value in 13.5px semibold Serif, and the
  boundary to one strong hairline. Colour may touch only a signal value. Never
  use this pattern for actions, filters, long identifiers, primary metrics, or
  as a replacement for a table.

## Layout

A masthead (brand, snapshot picker, search, actions) over a slim IDE-style
side navigation (~176px: icon+label rows, tree subtrees, Settings pinned to
the bottom) beside the workspace, with a status footer. Every route owns one
primary working surface. The Estate hierarchy is the only persistent
secondary rail. A selected resource opens a dedicated, scrollable evidence
sheet with its complete stored properties, identity, findings, relationships,
and identifiers. Identity, tags, ARM identifiers, and snapshot provenance sit
inline before the property evidence rather than occupying a fixed side column,
so every long table retains the full sheet width. The five resource summary
facts sit in quiet, compact header capsules rather than a separate stat band.
Capsules wrap together below the identity block when the header narrows; they
do not squeeze the title or force action labels to wrap.
Shape-driven rendering turns
fields into inline property rows and collections into data tables instead of
exposing JSON or an object tree. It never narrows the estate table. Finding
evidence remains an on-demand drawer. In Relationships, groups open their maps
directly, resources open their records, and aggregate tiles expand in place.
Subscription and resource-group disclosure is independent from selection; an
opened group reveals its resources in place. Azure entities use
the vendored Azure artwork at every depth, with Lucide reserved for controls,
navigation, and signals.

## Theme behaviour

A tri-state in Settings: **System** (default — follows `prefers-color-scheme`)
· Light · Dark. An explicit choice pins `data-theme` on `<html>` and persists
in `localStorage`; components never branch on the mode — only tokens change.
The Cytoscape stage reads its `--graph-*` tokens at build time and rebuilds
when the resolved theme changes.

## The graph, still

The topology keeps its invariants regardless of skin: native Azure service
artwork on transparent labels, deterministic layouts, boundary-anchored
round-taxi connectors with small arrows, containment never drawn, and honest
counts (drawn + folded + aggregated = total). In light mode it reads like the
report's printed diagrams; in dark, like the same plate at night.
