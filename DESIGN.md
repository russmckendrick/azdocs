# Design

The desktop app is a premium desktop data application: a dark navy
navigation frame in both modes, a cool neutral workspace, restrained Azure
blue, Inter throughout, subtle depth, and colour reserved for data and
signals. Light is canonical; dark is a genuine re-tuning of the same roles —
matte, layered and quiet — never an inversion. This palette belongs to the
desktop app only. PDF and DOCX themes remain the data-driven document themes.

This file used to restate that in full. It doesn't any more — three copies of
the same palette is how they drift.

| Where | What it holds |
|---|---|
| [`desktop/src/styles.css`](desktop/src/styles.css) | **The tokens.** The only place a colour value is written down. |
| [`docs/reference/design.md`](docs/reference/design.md) | **The language.** Frame, rules, rationale, component anatomy — the authoritative sheet. |
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

- **Selection is a quiet `--az-selected` fill.** Never a coloured bar or
  left-edge accent — the sidebar's selected row is a soft blue emphasis.
- **Blue is for actions, selection, links and interactive series.** It does
  not colour every heading or icon; saturated colour is otherwise the
  category set, the severity set and the three named accents, each meaning
  one thing everywhere.
- **No numbering chrome.** Figures get a caption, not a number.
- **Nothing below 11px** at the default text scale; type and spacing are
  `rem` off a 12px base so the reader can scale them, hairlines stay `px`.
- **Cards are the Overview's idiom, not the app's.** Estate is tree + table,
  Map a canvas, Inventory and Findings filters + tables, records an evidence
  sheet. Stay dense; polish comes from hierarchy, not whitespace.
- **Azure artwork lives in the workspace.** Navigation uses one outline
  icon set (Lucide); official Azure icons identify real resources in tables,
  cards, the canvas and records at the documented sizes.
- **Dark re-tunes, it does not invert.** Never hardcode a canvas colour; the
  Cytoscape stage reads `--graph-*`/`--kind-*` at build time and rebuilds on
  theme change, and both dark blocks in `styles.css` must agree.
