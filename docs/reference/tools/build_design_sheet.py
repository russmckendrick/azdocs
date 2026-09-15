#!/usr/bin/env python3
"""Render docs/reference/design.html from the desktop token layer.

The sheet exists because Markdown cannot show a colour: `design.md` describes
the language, this shows it. But a hand-maintained copy of every hex value is a
third place for them to live (after `desktop/src/styles.css` and the app
itself), and the one guaranteed to rot first.

So the values come from `styles.css` and only the editorial text — the role each
token plays, the type scale captions, the component anatomy — lives here.

    python3 docs/reference/tools/build_design_sheet.py
    python3 docs/reference/tools/build_design_sheet.py --check   # CI-style

`--check` rebuilds in memory and fails if the checked-in file differs, so a
token edit that was never re-rendered is visible.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
STYLES = ROOT / "desktop" / "src" / "styles.css"
MARKS = ROOT / "docs" / "marks" / "manifest.json"
TARGET = ROOT / "docs" / "reference" / "design.html"

# (css custom property, name shown on the sheet, light role, dark role)
PALETTE = [
    ("paper", "paper", "cool-grey ground", "charcoal ground"),
    ("surface", "surface", "raised panels, grids", "raised panels, grids"),
    ("evidence", "evidence", "JSON blocks, selection fill", "JSON blocks, selection fill"),
    ("ink", "ink", "headings and high-contrast anchors", "cool white headings and anchors"),
    ("body", "body", "running text", "running text"),
    ("muted", "muted", "secondary text", "secondary text"),
    ("line-strong", "hairline-strong", "panel borders", "panel borders"),
    ("line", "hairline", "row separators", "row separators"),
    ("accent", "accent", "links, selection, single-hue charts", "links, selection, single-hue charts"),
    ("coral", "coral", "high severity, risk", "high severity, risk"),
    ("amber", "amber", "medium severity, warnings", "medium severity, warnings"),
    ("green", "green", "healthy, resolved, complete", "healthy, resolved, complete"),
]

ATMOSPHERE = [
    ("ground-wash", "app ground + chrome"),
    ("surface-wash", "panels + controls"),
    ("evidence-wash", "selected evidence"),
    ("action-wash", "primary action"),
]

# The fixed category order, mirroring `category_color()` in src/diagram/icons.rs.
CATEGORIES = [
    ("cat-compute", "compute"),
    ("cat-networking", "network"),
    ("cat-storage", "storage*"),
    ("cat-databases", "database"),
    ("cat-appservices", "app svc"),
    ("cat-other", "other"),
]

# (manifest key, caption, the field it is shown on, minimum size from
# docs/marks/USAGE.md). The paths come from docs/marks/manifest.json so a
# renamed asset fails here rather than rendering a broken image.
MARK_ASSETS = [
    ("markPrimary", "Mark", "paper", "24px tall"),
    ("markReversed", "Reversed", "ink", "24px tall"),
    ("markMonoInk", "Mono ink", "paper", "one colour"),
    ("appIconLight", "App icon", "evidence", "32px square"),
]
LOCKUP_ASSETS = [
    ("lockupPrimary", "Lockup", "paper", "140px wide"),
    ("lockupReversed", "Lockup reversed", "ink", "140px wide"),
]

SEVERITIES = [
    ("coral", "High", "coral · always with the word, never colour alone"),
    ("amber", "Medium", "amber"),
    ("muted", "Low", "neutral"),
    ("accent", "Info", "accent"),
]

# (family, size, weight, sample, caption, extra sample style, extra caption style)
TYPE_SCALE = [
    ("serif", 30, 600, "Display 30", "Plex Serif 600 · view titles, hero numerals", " letter-spacing: -0.01em;", ""),
    ("serif", 19, 600, "Chapter 19", "Plex Serif 600 · section &amp; panel headings", "", ""),
    ("sans", 14, 400, "Body 14 — descriptions, ledes and supporting evidence read at this size.", "Plex Sans 400", "", " flex-shrink: 0;"),
    ("sans", 12.5, 400, "Table 12.5 — ledger rows, inspector detail", "Plex Sans 400/600", "", ""),
    ("caps", 11, 600, "Label 11 · caps", "Plex Sans 600 · column heads, metadata", "", ""),
    ("mono", 12, 400, "evidence-12 · /subscriptions/…/rg-prod-data", "Plex Mono · ARM ids, tags, deltas", "", ""),
]


def block(css: str, selector: str) -> dict[str, str]:
    """Custom properties declared in the first rule matching `selector`."""
    start = css.index(selector)
    depth, i = 0, css.index("{", start)
    opened = i
    while True:
        if css[i] == "{":
            depth += 1
        elif css[i] == "}":
            depth -= 1
            if depth == 0:
                break
        i += 1
    body = css[opened : i + 1]
    return {name: value.strip() for name, value in re.findall(r"--([\w-]+):\s*([^;]+);", body)}


def load_tokens() -> tuple[dict[str, str], dict[str, str]]:
    css = STYLES.read_text(encoding="utf-8")
    light = block(css, ":root {")
    toggled = block(css, ':root[data-theme="dark"]')
    system = block(css, "@media (prefers-color-scheme: dark)")

    # AGENTS.md: dark is a 1:1 remap, and the toggle must win in both
    # directions — which only holds if both dark blocks say the same thing.
    drift = {
        name: (system.get(name), toggled.get(name))
        for name in set(system) | set(toggled)
        if system.get(name) != toggled.get(name)
    }
    if drift:
        lines = "\n".join(
            f"  --{name}: media={pair[0]!r} toggle={pair[1]!r}"
            for name, pair in sorted(drift.items())
        )
        raise SystemExit(
            "styles.css dark blocks disagree; the media query and the "
            f"[data-theme=\"dark\"] rule must define the same values:\n{lines}"
        )
    return light, toggled


def load_marks() -> dict[str, str]:
    """Asset paths from the mark manifest, relative to docs/reference/."""
    manifest = json.loads(MARKS.read_text(encoding="utf-8"))
    svg = manifest.get("svg", {})
    wanted = [key for key, *_ in MARK_ASSETS + LOCKUP_ASSETS]
    missing = [key for key in wanted if key not in svg]
    if missing:
        raise SystemExit(
            f"docs/marks/manifest.json has no svg entry for: {', '.join(missing)}. "
            "Rename it back, or update build_design_sheet.py."
        )
    paths = {}
    for key in wanted:
        asset = ROOT / "docs" / "marks" / svg[key]
        if not asset.exists():
            raise SystemExit(f"{svg[key]} is in the manifest but missing on disk.")
        paths[key] = f"../marks/{svg[key]}"
    return paths


def hexes(tokens: dict[str, str], mode: str) -> dict[str, str]:
    missing = [name for name, *_ in PALETTE if name not in tokens]
    missing += [name for name, _ in ATMOSPHERE if name not in tokens]
    missing += ["action-text"] if "action-text" not in tokens else []
    missing += [name for name, _ in CATEGORIES if name not in tokens and mode == "light"]
    if missing:
        raise SystemExit(
            f"styles.css has no {mode} value for: {', '.join(sorted(set(missing)))}. "
            "Add it, or drop it from build_design_sheet.py."
        )
    return tokens


def atmosphere_tiles(light: dict[str, str], dark: dict[str, str], mode: str) -> str:
    values = light if mode == "light" else {**light, **dark}
    return "\n".join(
        f'<div style="min-height: 54px; padding: 9px 11px; border: 1px solid {values["line-strong"]}; '
        f'border-radius: 5px; background: {values[token]}; color: {values["ink"]}; display: flex; '
        f'align-items: end"><span style="font-size: 11px; font-weight: 600">{label}</span></div>'
        for token, label in ATMOSPHERE
    )


def swatch_rows(light: dict[str, str], dark: dict[str, str], mode: str) -> str:
    values = light if mode == "light" else {**light, **dark}
    out = []
    for index, (token, label, light_role, dark_role) in enumerate(PALETTE):
        role = light_role if mode == "light" else dark_role
        last = ' style="border-bottom: none"' if index == len(PALETTE) - 1 else ""
        out.append(
            f'          <div class="swatch-row"{last}>'
            f'<span class="swatch" style="background: {values[token]}"></span>'
            f'<span class="sw-name">{label}</span>'
            f'<span class="sw-hex">{values[token]}</span>'
            f'<span class="sw-role">{role}</span></div>'
        )
    return "\n".join(out)


def render() -> str:
    light, dark = load_tokens()
    hexes(light, "light")
    merged_dark = {**light, **dark}
    marks = load_marks()

    fields = {
        "paper": (light["paper"], light["line-strong"]),
        "ink": (light["ink"], light["ink"]),
        "evidence": (light["evidence"], light["line-strong"]),
    }

    def mark_tile(key: str, caption: str, field: str, note: str, height: int) -> str:
        background, border = fields[field]
        return (
            '        <div style="display: flex; flex-direction: column; gap: 6px">'
            f'<div style="display: flex; align-items: center; justify-content: center; '
            f'height: {height + 28}px; background: {background}; border: 1px solid {border}; '
            f'border-radius: 6px; padding: 14px">'
            f'<img src="{marks[key]}" alt="{caption}" style="height: {height}px">'
            "</div>"
            f'<div style="display: flex; align-items: baseline; gap: 8px">'
            f'<span style="font-size: 12px; font-weight: 600">{caption}</span>'
            f'<span class="anno">{note}</span></div></div>'
        )

    mark_tiles = "\n".join(
        mark_tile(key, caption, field, note, 40) for key, caption, field, note in MARK_ASSETS
    )
    lockup_tiles = "\n".join(
        mark_tile(key, caption, field, note, 30) for key, caption, field, note in LOCKUP_ASSETS
    )

    cat_bars = "\n".join(
        f'            <div style="flex: 1; height: 26px; background: {light[token]}; border-radius: 3px"></div>'
        for token, _ in CATEGORIES
    )
    cat_labels = "".join(f'<span style="flex: 1">{label}</span>' for _, label in CATEGORIES)
    dark_storage = merged_dark["cat-storage"]

    severity_rows = "\n".join(
        f'            <div style="display: flex; align-items: center; gap: 8px; font-size: 12px">'
        f'<span style="width: 10px; height: 10px; border-radius: 2px; background: {light[token]}"></span>'
        f'<span style="font-weight: 600; width: 58px">{label}</span>'
        f'<span class="anno">{note}</span></div>'
        for token, label, note in SEVERITIES
    )

    scale_rows = []
    for index, (family, size, weight, sample, note, extra, note_extra) in enumerate(TYPE_SCALE):
        border = "" if index == len(TYPE_SCALE) - 1 else f" border-bottom: 1px solid {light['line']};"
        if family == "serif":
            span = f'<span class="serif" style="font-size: {size}px; font-weight: {weight};{extra}">{sample}</span>'
        elif family == "mono":
            span = f'<span class="mono" style="font-size: {size}px; color: {light["body"]};{extra}">{sample}</span>'
        elif family == "caps":
            span = (
                f'<span style="font-size: {size}px; font-weight: {weight}; letter-spacing: 0.12em; '
                f'text-transform: uppercase; color: {light["faint"]};{extra}">{sample}</span>'
            )
        else:
            span = f'<span style="font-size: {size}px; color: {light["body"]};{extra}">{sample}</span>'
        scale_rows.append(
            f'        <div style="display: flex; align-items: baseline; gap: 16px; padding: 7px 0;{border}">'
            f'{span}<span class="anno" style="margin-left: auto;{note_extra}">{note}</span></div>'
        )
    scale = "\n".join(scale_rows)

    return f"""<!doctype html>
<!-- @generated by docs/reference/tools/build_design_sheet.py — do not edit by
     hand. Colour values come from desktop/src/styles.css; the prose lives in
     the generator. Re-run it after changing either. -->
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>azdocs · Design language</title>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Serif:wght@400;500;600&amp;family=IBM+Plex+Sans:wght@400;500;600&amp;family=IBM+Plex+Mono:wght@400;500&amp;display=swap">
  <style>
    body {{ margin: 0; font-family: "IBM Plex Sans", "Segoe UI", Georgia, system-ui, sans-serif; color: {light['ink']}; background: {light['paper']}; }}
    .page {{ max-width: 1440px; margin: 0 auto; padding: 28px 56px 40px; box-sizing: border-box; display: flex; flex-direction: column; gap: 20px; }}
    a {{ color: {light['accent']}; }} a:hover {{ color: {light['accent-hover']}; }}
    .serif {{ font-family: "IBM Plex Serif", Georgia, serif; }}
    .mono {{ font-family: "IBM Plex Mono", ui-monospace, monospace; }}
    .sec-title {{ font-family: "IBM Plex Serif", Georgia, serif; font-size: 19px; font-weight: 600; margin: 0; }}
    .sec-note {{ font-size: 12.5px; color: {light['muted']}; }}
    .swatch-row {{ display: flex; align-items: center; gap: 10px; padding: 5px 0; border-bottom: 1px solid {light['line']}; }}
    .swatch {{ width: 30px; height: 20px; border-radius: 3px; border: 1px solid {light['line-strong']}; flex-shrink: 0; }}
    .sw-name {{ font-size: 12px; font-weight: 600; width: 104px; }}
    .sw-hex {{ font-family: "IBM Plex Mono", ui-monospace, monospace; font-size: 11px; color: {light['muted']}; width: 66px; }}
    .sw-role {{ font-size: 11.5px; color: {light['faint']}; flex: 1; }}
    .dk .swatch-row {{ border-bottom: 1px solid {merged_dark['line']}; }}
    .dk .swatch {{ border: 1px solid {merged_dark['line-strong']}; }}
    .dk .sw-name {{ color: {merged_dark['ink']}; }}
    .dk .sw-hex {{ color: {merged_dark['muted']}; }}
    .dk .sw-role {{ color: {merged_dark['faint']}; }}
    .anno {{ font-family: "IBM Plex Mono", ui-monospace, monospace; font-size: 10.5px; color: {light['faint']}; }}
    .comp-label {{ font-size: 10.5px; font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase; color: {light['faint']}; }}
    .columns {{ display: grid; grid-template-columns: 1.1fr 1fr; gap: 44px; }}
    @media (max-width: 1100px) {{ .columns {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
<div class="page">

  <header style="display: flex; align-items: baseline; gap: 18px; border-bottom: 2px solid {light['ink']}; padding-bottom: 14px; flex-wrap: wrap">
    <span class="serif" style="font-size: 26px; font-weight: 600; letter-spacing: -0.01em">azdocs · Design language</span>
    <span style="flex: 1"></span>
    <span style="font-size: 12.5px; color: {light['muted']}">Desktop-only cool-grey-to-Azure atmosphere — IBM Plex, editorial structure, saturated colour reserved for data, signals and actions. Tokens: <span class="mono" style="font-size: 11px">desktop/src/styles.css</span> · prose: <a href="design.md">design.md</a></span>
  </header>

  <div class="columns">

    <!-- LEFT: palette -->
    <div style="display: flex; flex-direction: column; gap: 14px">
      <div style="display: flex; align-items: baseline; gap: 12px"><h2 class="sec-title">Palette</h2><span class="sec-note">two modes, one structure — every light token has exactly one dark counterpart</span></div>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0; border: 1px solid {light['line-strong']}; border-radius: 6px; overflow: hidden">
        <div style="padding: 14px 18px; background: {light['paper']}">
          <div class="comp-label" style="margin-bottom: 8px">Light · default</div>
{swatch_rows(light, dark, "light")}
        </div>
        <div class="dk" style="padding: 14px 18px; background: {merged_dark['paper']}">
          <div class="comp-label" style="margin-bottom: 8px; color: {merged_dark['faint']}">Dark · night reading</div>
{swatch_rows(light, dark, "dark")}
        </div>
      </div>

      <div>
        <div class="comp-label" style="margin-bottom: 8px">Atmosphere · gradients carry large regions, never data values</div>
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px">
          <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 7px">
{atmosphere_tiles(light, dark, "light")}
          </div>
          <div class="dk" style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 7px; padding: 8px; margin: -8px; border-radius: 6px; background: {merged_dark['paper']}">
{atmosphere_tiles(light, dark, "dark")}
          </div>
        </div>
      </div>

      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px">
        <div>
          <div class="comp-label" style="margin-bottom: 8px">Chart categories · CVD-validated both modes</div>
          <div style="display: flex; gap: 6px; margin-bottom: 6px">
{cat_bars}
          </div>
          <div style="display: flex; gap: 6px" class="anno">
            {cat_labels}
          </div>
          <div class="sec-note" style="margin-top: 6px">Fixed order, never cycled — the top slots of the app's <span class="mono" style="font-size: 11px">category_color()</span> set. Charts fold the tail into Other; type dots elsewhere may use the full set. *storage deepens to <span class="mono" style="font-size: 11px">{dark_storage}</span> in dark. Bars are always direct-labelled.</div>
        </div>
        <div>
          <div class="comp-label" style="margin-bottom: 8px">Severity · status, never decorative</div>
          <div style="display: flex; flex-direction: column; gap: 5px">
{severity_rows}
          </div>
          <div class="sec-note" style="margin-top: 8px">Footnote daggers (<span style="color: {light['coral']}; font-weight: 600">†</span>) mark rows referenced by open findings, in every table of the app.</div>
        </div>
      </div>

      <div style="display: flex; align-items: baseline; gap: 12px; margin-top: 4px"><h2 class="sec-title">Mark</h2><span class="sec-note">a rounded topology A — three nodes and the connectors between them</span></div>
      <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 14px">
{mark_tiles}
      </div>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px">
{lockup_tiles}
      </div>
      <div class="sec-note">The node diameter is the clear-space unit <span class="mono" style="font-size: 11px">x</span> — keep at least one <span class="mono" style="font-size: 11px">x</span> clear on every side. Never crop the mark; only the supplied background motifs crop it, deliberately. Full rules: <a href="../marks/USAGE.md">marks/USAGE.md</a>. Native app icons are derived from the light app icon by <span class="mono" style="font-size: 11px">pnpm run icons</span>, not drawn by hand.</div>
    </div>

    <!-- RIGHT: type + structure -->
    <div style="display: flex; flex-direction: column; gap: 14px; border-left: 1px solid {light['line-strong']}; padding-left: 44px">
      <div style="display: flex; align-items: baseline; gap: 12px"><h2 class="sec-title">Type</h2><span class="sec-note">IBM Plex — the family the PDF already vendors</span></div>
      <div style="display: flex; flex-direction: column">
{scale}
      </div>
      <div class="sec-note">Nothing below 11px — the old app's 6–10px floor is retired. Serif carries identity; sans does the work; mono marks machine-shaped values.</div>

      <div style="display: flex; align-items: baseline; gap: 12px; margin-top: 4px"><h2 class="sec-title">Structure</h2><span class="sec-note">rules over boxes, tone over shadow</span></div>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; font-size: 12px; color: {light['body']}">
        <div>
          <div style="border-top: 2px solid {light['ink']}; padding-top: 5px; margin-bottom: 8px">2px ink rule — reserved for high-value summary anchors</div>
          <div style="border-top: 1px solid {light['line-strong']}; padding-top: 5px; margin-bottom: 8px">1px strong hairline — shell rails, record headers, panel edges</div>
          <div style="border-top: 1px solid {light['line']}; padding-top: 5px">1px hairline — sections, table rows, list separators</div>
        </div>
        <div>
          <div style="margin-bottom: 6px">Spacing rhythm: <span class="mono">4 · 8 · 12 · 20 · 32 · 56</span> (56 = page margin)</div>
          <div style="margin-bottom: 6px">Radii: <span class="mono">3px</span> controls &amp; tag chips · <span class="mono">6px</span> panels · fully rounded summary capsules — one purposeful exception to the near-square evidence workspace language</div>
          <div style="margin-bottom: 6px">One working surface per route. Resource records use compact header facts and a full evidence sheet with inline context, property rows, and collection tables — never a fixed detail sidebar. Findings use an on-demand drawer; Relationships navigates directly from groups to maps and resources to records.</div>
          <div style="margin-bottom: 6px">Subscriptions and resource groups disclose independently from selection; Azure entities use the vendored Azure artwork at every depth.</div>
          <div>Large collections use labelled selectors and honest progressive batches. Figures end with a caption; no numbering, anywhere.</div>
        </div>
      </div>

      <div style="display: flex; align-items: baseline; gap: 12px; margin-top: 4px"><h2 class="sec-title">Components</h2><span class="sec-note">same anatomy in both modes</span></div>
      <div style="border: 1px solid {light['line-strong']}; border-radius: 6px; overflow: hidden">
        <div style="padding: 12px 16px; background: {light['surface']}; display: flex; align-items: center; gap: 12px; flex-wrap: wrap">
          <span style="font-size: 12px; font-weight: 600; color: {light['action-text']}; background: {light['action-wash']}; padding: 7px 14px; border-radius: 3px">Primary action</span>
          <span style="font-size: 12px; font-weight: 600; color: {light['body']}; border: 1px solid {light['line-strong']}; padding: 6px 13px; border-radius: 3px">Quiet action</span>
          <span style="font-size: 12.5px; font-weight: 600; color: {light['accent']}; border-bottom: 2px solid {light['accent']}; padding-bottom: 4px">Active tab · 12</span>
          <span style="font-size: 12.5px; color: {light['muted']}; padding-bottom: 4px">Tab · 8</span>
          <span class="mono" style="font-size: 11px; background: {light['evidence']}; border: 1px solid {light['line']}; padding: 3px 8px; border-radius: 3px">environment=prod</span>
          <span style="display: inline-flex; align-items: baseline; gap: 7px; padding: 5px 8px 6px; border: 1px solid {light['line-strong']}; border-radius: 999px"><span style="font-size: 11px; font-weight: 600; letter-spacing: 0.06em; color: {light['faint']}">LOCATION</span><strong class="serif" style="font-size: 13.5px">UK South</strong></span>
          <span style="font-size: 12px; color: {light['body']}; border: 1px solid {light['line-strong']}; border-radius: 3px; padding: 6px 10px; background: {light['paper']}">Search evidence… <span class="mono" style="font-size: 11px; color: {light['faintest']}">⌘K</span></span>
        </div>
        <div style="padding: 12px 16px; background: {merged_dark['paper']}; display: flex; align-items: center; gap: 12px; flex-wrap: wrap">
          <span style="font-size: 12px; font-weight: 600; color: {merged_dark['action-text']}; background: {merged_dark['action-wash']}; padding: 7px 14px; border-radius: 3px">Primary action</span>
          <span style="font-size: 12px; font-weight: 600; color: {merged_dark['body']}; border: 1px solid {merged_dark['line-strong']}; padding: 6px 13px; border-radius: 3px">Quiet action</span>
          <span style="font-size: 12.5px; font-weight: 600; color: {merged_dark['accent']}; border-bottom: 2px solid {merged_dark['accent']}; padding-bottom: 4px">Active tab · 12</span>
          <span style="font-size: 12.5px; color: {merged_dark['muted']}; padding-bottom: 4px">Tab · 8</span>
          <span class="mono" style="font-size: 11px; background: {merged_dark['evidence']}; border: 1px solid {merged_dark['line']}; color: {merged_dark['body']}; padding: 3px 8px; border-radius: 3px">environment=prod</span>
          <span style="display: inline-flex; align-items: baseline; gap: 7px; padding: 5px 8px 6px; color: {merged_dark['ink']}; border: 1px solid {merged_dark['line-strong']}; border-radius: 999px"><span style="font-size: 11px; font-weight: 600; letter-spacing: 0.06em; color: {merged_dark['faint']}">LOCATION</span><strong class="serif" style="font-size: 13.5px">UK South</strong></span>
          <span style="font-size: 12px; color: {merged_dark['body']}; border: 1px solid {merged_dark['line-strong']}; border-radius: 3px; padding: 6px 10px; background: {merged_dark['paper']}">Search evidence… <span class="mono" style="font-size: 11px; color: {merged_dark['faintest']}">⌘K</span></span>
        </div>
      </div>
      <div class="sec-note">Summary capsules hold two to six short, read-only orientation facts in spare header or toolbar space. They wrap together, colour only signal values, and never replace controls, primary metrics, long evidence, tables, or ledgers. Theme is a tri-state in Settings — <span style="font-weight: 600">System</span> (default, follows the OS) · Light · Dark. The primary button keeps a higher-contrast Azure wash in both modes; category and severity colours are re-validated per surface, never auto-flipped. Selection anywhere is a quiet <span class="mono" style="font-size: 11px">--evidence</span> fill — never a coloured bar.</div>
    </div>
  </div>

  <footer style="display: flex; align-items: center; gap: 14px; border-top: 2px solid {light['ink']}; padding-top: 10px; font-size: 11.5px; color: {light['faint']}; flex-wrap: wrap">
    <span>Implemented in <span class="mono" style="font-size: 11px">desktop/src/styles.css</span> · prose version at <a href="design.md">design.md</a> · generated by <span class="mono" style="font-size: 11px">docs/reference/tools/build_design_sheet.py</span>.</span>
    <span style="margin-left: auto">azdocs · Desktop design language</span>
  </footer>
</div>
</body>
</html>
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if the checked-in sheet differs from a fresh render",
    )
    args = parser.parse_args()

    rendered = render()
    if args.check:
        current = TARGET.read_text(encoding="utf-8") if TARGET.exists() else ""
        if current != rendered:
            print(
                f"{TARGET.relative_to(ROOT)} is stale — re-run "
                "`python3 docs/reference/tools/build_design_sheet.py`.",
                file=sys.stderr,
            )
            return 1
        print(f"{TARGET.relative_to(ROOT)} is current.")
        return 0

    TARGET.write_text(rendered, encoding="utf-8")
    print(f"wrote {TARGET.relative_to(ROOT)} from {STYLES.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
