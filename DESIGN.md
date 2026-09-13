# Design

The desktop app combines a balanced Overview dashboard with focused evidence
workspaces. IBM Plex Serif carries identity, Plex Sans does the work, and Plex
Mono marks machine-shaped evidence. Light mode is paper, ink and hairline
rules; dark mode remaps the same composition to warm charcoal.

This file used to restate that in full. It doesn't any more — three copies of
the same palette is how they drift.

| Where | What it holds |
|---|---|
| [`desktop/src/styles.css`](desktop/src/styles.css) | **The tokens.** The only place a colour value is written down. |
| [`docs/reference/design.md`](docs/reference/design.md) | **The language.** Rules, rationale, component anatomy — the authoritative sheet. |
| [`docs/reference/design.html`](docs/reference/design.html) | **The swatches.** Generated from `styles.css`, because Markdown cannot show a colour. |
| [`docs/development/desktop-relationships.md`](docs/development/desktop-relationships.md) | **The Map workspace contract**, including its regression rules. |

Regenerate the swatch sheet after changing a token:

```sh
python3 docs/reference/tools/build_design_sheet.py
python3 docs/reference/tools/build_design_sheet.py --check   # is it current?
```

## The rules that get broken most

Stated here because they are the ones a plausible-looking change tends to
undo. Everything else is in [`docs/reference/design.md`](docs/reference/design.md).

- **Colour is for data and signals only** — the category set and the severity
  set. Chrome is paper, ink and hairlines.
- **Selection is a quiet `--evidence` fill.** Never a coloured bar.
- **No numbering chrome.** Figures get a caption, not a number.
- **Nothing below 11px.**
- **Overview has four KPI cards and responsive chart panels.** This approved
  exception to the stat-strip pattern opens details in focused dialogs, with
  links to matching results. The authoritative sheet records its scope and
  history rules.
- **The masthead identity is the azdocs logo.** Side navigation uses Azure
  artwork and can minimise to a persistent, labelled 68px icon rail.
- **Dark is a 1:1 token remap**, not a second composition. Never hardcode a
  canvas colour; the Cytoscape stage reads `--graph-*`/`--kind-*` at build time
  and rebuilds on theme change.
