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
- Spacing rhythm `4 · 8 · 12 · 20 · 32`; radii `3px` (chips, controls) and
  `6px` (panels) — near-square, print-like.
- The **stat strip** is the summary idiom: serif numerals between rules, one
  cell per measure, coral only when the measure is a risk.
- Side navigation is a slim IDE-style list (~176px): icon + label rows,
  tree subtrees with guide lines, Settings pinned to the bottom, a mono coral
  count as the Findings badge.

## Components

- **Primary button**: ink block, paper text — inverts with the mode.
- **Quiet button**: surface fill, strong hairline border.
- **Active tab**: 2px accent underline (tabs are the one place the accent
  underlines; navigation never does).
- **Tag chip**: mono on evidence fill; a **missing** required tag is a dashed
  coral outline.
- **Evidence blocks** (stored JSON, ARM ids): mono on `--evidence` with a
  hairline border, never a dark backplate in light mode.
- **Charts**: horizontal bars direct-labelled with the value; one hue for one
  measure, category colours only for categorical identity; captions are one
  short italic line.
