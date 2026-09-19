# Design

The desktop app combines a balanced Overview dashboard with focused evidence
workspaces. IBM Plex Serif carries identity, Plex Sans does the work, and Plex
Mono marks machine-shaped evidence. Light mode is paper, ink and hairline
rules over a soft cool-grey-to-Azure atmosphere; dark mode remaps the same
composition from charcoal into deep Azure. This palette belongs to the desktop
app only. PDF and DOCX themes remain the data-driven document themes.

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

- **Ambient colour stays soft** — cool grey/Azure washes may carry large chrome
  and surface regions. Saturated colour is still reserved for the category and
  severity sets, focus and primary actions.
- **Selection is a quiet `--evidence` fill.** Never a coloured bar.
- **No numbering chrome.** Figures get a caption, not a number.
- **Nothing below 11px** at the default text scale; type and spacing are
  `rem` off a 13px base so the reader can scale them, hairlines stay `px`.
- **Overview has four KPI cards and responsive chart panels.** This approved
  exception to the stat-strip pattern opens details in focused dialogs, with
  links to matching results. The authoritative sheet records its scope and
  history rules.
- **The masthead identity is the azdocs logo.** Side navigation uses Azure
  artwork and can minimise to a persistent, labelled 68px icon rail.
- **Dark is a 1:1 token remap**, not a second composition. Never hardcode a
  canvas colour; the Cytoscape stage reads `--graph-*`/`--kind-*` at build time
  and rebuilds on theme change.
