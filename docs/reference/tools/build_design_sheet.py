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
    ("az-canvas", "canvas", "workspace ground, never pure white", "near-black navy ground"),
    ("az-surface-1", "surface-1", "cards, panels, command bar", "cards, panels, command bar"),
    ("az-surface-2", "surface-2", "raised controls, footers", "raised controls, footers"),
    ("az-surface-3", "surface-3", "evidence blocks, tracks, chips", "evidence blocks, tracks, chips"),
    ("az-hover", "hover", "table and list rows under the pointer", "rows under the pointer"),
    ("az-selected", "selected", "the selected row, tab or control", "the selected row, tab or control"),
    ("az-text-primary", "text-primary", "headings, values, running text", "headings, values, running text"),
    ("az-text-secondary", "text-secondary", "supporting copy, icons, captions", "supporting copy, icons, captions"),
    ("az-text-muted", "text-muted", "placeholders, micro metadata", "placeholders, micro metadata"),
    ("az-border", "border", "card and control boundaries", "boundaries carry the depth"),
    ("az-border-subtle", "border-subtle", "row separators", "row separators"),
    ("az-border-strong", "border-strong", "hover and focus boundaries", "hover and focus boundaries"),
    ("az-primary", "primary", "links, selected navigation, series", "brighter so it does not sink"),
    ("az-primary-hover", "primary-hover", "interactive hover", "interactive hover"),
    ("az-primary-soft", "primary-soft", "soft blue tint", "soft blue tint"),
    ("az-success", "success", "healthy, complete", "healthy, complete"),
    ("az-warning", "warning", "medium severity, partial", "medium severity, partial"),
    ("az-danger", "danger", "high severity, failed", "high severity, failed"),
    ("az-info", "info", "low severity, informational", "low severity, informational"),
    ("az-neutral", "neutral", "info severity, quiet signals", "info severity, quiet signals"),
]

# The navigation frame is navy in both modes; these draw on top of it.
SIDEBAR = [
    ("az-sidebar", "sidebar"),
    ("az-sidebar-elevated", "elevated"),
    ("az-sidebar-hover", "hover"),
    ("az-sidebar-selected", "selected"),
]

# Supporting accents: each stands for one thing everywhere in the app.
ACCENTS = [
    ("az-primary-fill", "action", "primary buttons, both modes"),
    ("az-resource", "resource", "resource counts and KPI"),
    ("az-governance", "governance", "tags and coverage"),
    ("az-relationship", "relationship", "stored connections"),
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
    ("markPrimary", "Mark", "canvas", "24px tall"),
    ("markReversed", "Reversed", "sidebar", "24px tall"),
    ("markMonoInk", "Mono ink", "canvas", "one colour"),
    ("appIconLight", "App icon", "surface", "32px square"),
]
LOCKUP_ASSETS = [
    ("lockupPrimary", "Lockup", "canvas", "140px wide"),
    ("lockupReversed", "Lockup reversed", "sidebar", "140px wide"),
]

SEVERITIES = [
    ("severity-high", "High", "danger · always with the word and an icon, never colour alone"),
    ("severity-medium", "Medium", "warning"),
    ("severity-low", "Low", "info"),
    ("severity-info", "Info", "neutral"),
]

# (size, weight, sample, caption, extra sample style)
TYPE_SCALE = [
    (24, 650, "Page title 24", "Inter 650 · one per route, letter-spacing −0.02em", " letter-spacing: -0.02em;"),
    (26, 650, "32,768", "KPI value 26 · Inter 650, tabular numerals", " letter-spacing: -0.02em; font-variant-numeric: tabular-nums;"),
    (16, 650, "Section heading 16", "Inter 650", " letter-spacing: -0.01em;"),
    (14, 600, "Card heading 14", "Inter 600 · panel and card titles", ""),
    (13, 400, "Body 13 — descriptions and supporting copy read at this size.", "Inter 400 · line-height 1.4", ""),
    (12, 500, "UI control 12 — buttons, selects, navigation, table cells", "Inter 500 · the dense working size", ""),
    (11, 400, "Secondary 11 — table heads, captions, secondary lines", "Inter 400/600 · the floor", ""),
    (11, 600, "LABEL 11 · TRACKED CAPS", "Inter 600 · letter-spacing 0.08em, metadata only", " letter-spacing: 0.08em; text-transform: uppercase;"),
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

    # AGENTS.md: dark is one re-tuning of the same roles, and the toggle must
    # win in both directions — which only holds if both dark blocks agree.
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


def resolve(tokens: dict[str, str], name: str) -> str:
    """A token's literal value, following `var(--other)` aliases."""
    value = tokens[name]
    seen = {name}
    while True:
        alias = re.fullmatch(r"var\(--([\w-]+)\)", value)
        if not alias:
            return value
        target = alias.group(1)
        if target in seen or target not in tokens:
            raise SystemExit(f"--{name} aliases --{target}, which styles.css does not define.")
        seen.add(target)
        value = tokens[target]


def check_tokens(tokens: dict[str, str], mode: str) -> None:
    wanted = [name for name, *_ in PALETTE + SIDEBAR + ACCENTS + SEVERITIES]
    if mode == "light":
        wanted += [name for name, _ in CATEGORIES] + ["az-on-primary", "az-sidebar-text", "az-sidebar-text-strong", "az-sidebar-muted"]
    missing = [name for name in wanted if name not in tokens]
    if missing:
        raise SystemExit(
            f"styles.css has no {mode} value for: {', '.join(sorted(set(missing)))}. "
            "Add it, or drop it from build_design_sheet.py."
        )


def swatch_rows(values: dict[str, str], mode: str) -> str:
    out = []
    for index, (token, label, light_role, dark_role) in enumerate(PALETTE):
        role = light_role if mode == "light" else dark_role
        last = ' style="border-bottom: none"' if index == len(PALETTE) - 1 else ""
        out.append(
            f'          <div class="swatch-row"{last}>'
            f'<span class="swatch" style="background: {resolve(values, token)}"></span>'
            f'<span class="sw-name">{label}</span>'
            f'<span class="sw-hex">{resolve(values, token)}</span>'
            f'<span class="sw-role">{role}</span></div>'
        )
    return "\n".join(out)


def sidebar_tiles(values: dict[str, str]) -> str:
    text = resolve(values, "az-sidebar-text-strong")
    return "\n".join(
        f'<div style="min-height: 46px; padding: 8px 10px; border-radius: 7px; background: {resolve(values, token)}; '
        f'color: {text}; display: flex; flex-direction: column; justify-content: end; gap: 2px">'
        f'<span style="font-size: 11px; font-weight: 600">{label}</span>'
        f'<span class="mono" style="font-size: 10.5px; opacity: .7">{resolve(values, token)}</span></div>'
        for token, label in SIDEBAR
    )


def accent_tiles(values: dict[str, str]) -> str:
    return "\n".join(
        f'<div style="display: flex; align-items: center; gap: 8px; font-size: 12px">'
        f'<span style="width: 28px; height: 28px; border-radius: 8px; background: color-mix(in srgb, {resolve(values, token)} 14%, {resolve(values, "az-surface-1")}); '
        f'display: grid; place-items: center"><span style="width: 10px; height: 10px; border-radius: 3px; background: {resolve(values, token)}"></span></span>'
        f'<span style="font-weight: 600; width: 84px">{label}</span>'
        f'<span class="anno">{resolve(values, token)} · {note}</span></div>'
        for token, label, note in ACCENTS
    )


def render() -> str:
    light, dark_only = load_tokens()
    check_tokens(light, "light")
    # Dark only re-tunes what changes; aliases and mode-independent values
    # (the primary fill, the navy frame's text) fall through from light.
    dark = {**light, **dark_only}
    check_tokens(dark, "dark")
    marks = load_marks()

    fields = {
        "canvas": (resolve(light, "az-canvas"), resolve(light, "az-border")),
        "sidebar": (resolve(light, "az-sidebar"), resolve(light, "az-sidebar")),
        "surface": (resolve(light, "az-surface-3"), resolve(light, "az-border")),
    }

    def mark_tile(key: str, caption: str, field: str, note: str, height: int) -> str:
        background, border = fields[field]
        return (
            '        <div style="display: flex; flex-direction: column; gap: 6px">'
            f'<div style="display: flex; align-items: center; justify-content: center; '
            f'height: {height + 28}px; background: {background}; border: 1px solid {border}; '
            f'border-radius: 10px; padding: 14px">'
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
        f'            <div style="flex: 1; height: 26px; background: {resolve(light, token)}; border-radius: 5px"></div>'
        for token, _ in CATEGORIES
    )
    cat_labels = "".join(f'<span style="flex: 1">{label}</span>' for _, label in CATEGORIES)
    dark_storage = resolve(dark, "cat-storage")

    severity_rows = "\n".join(
        f'            <div style="display: flex; align-items: center; gap: 8px; font-size: 12px">'
        f'<span style="width: 10px; height: 10px; border-radius: 2px; background: {resolve(light, token)}"></span>'
        f'<span style="font-weight: 600; width: 58px">{label}</span>'
        f'<span class="anno">{note}</span></div>'
        for token, label, note in SEVERITIES
    )

    scale_rows = []
    for index, (size, weight, sample, note, extra) in enumerate(TYPE_SCALE):
        border = "" if index == len(TYPE_SCALE) - 1 else f" border-bottom: 1px solid {resolve(light, 'az-border-subtle')};"
        colour = resolve(light, "az-text-secondary") if size <= 11 else resolve(light, "az-text-primary")
        scale_rows.append(
            f'        <div style="display: flex; align-items: baseline; gap: 16px; padding: 7px 0;{border}">'
            f'<span style="font-size: {size}px; font-weight: {weight}; color: {colour};{extra}">{sample}</span>'
            f'<span class="anno" style="margin-left: auto; flex-shrink: 0">{note}</span></div>'
        )
    mono_row = (
        f'        <div style="display: flex; align-items: baseline; gap: 16px; padding: 7px 0">'
        f'<span class="mono" style="font-size: 11px; color: {resolve(light, "az-text-primary")}">/subscriptions/…/rg-prod-data · a7f21f53</span>'
        f'<span class="anno" style="margin-left: auto; flex-shrink: 0">Mono 11 · platform monospace, ARM ids, hashes, raw values</span></div>'
    )
    scale = "\n".join(scale_rows) + "\n" + mono_row

    L = lambda name: resolve(light, name)  # noqa: E731
    D = lambda name: resolve(dark, name)  # noqa: E731

    def components(v, mode: str) -> str:
        surface = resolve(v, "az-surface-1")
        return f"""        <div style="padding: 14px 16px; background: {resolve(v, 'az-canvas')}; display: flex; align-items: center; gap: 12px; flex-wrap: wrap; color: {resolve(v, 'az-text-primary')}">
          <span style="display: inline-flex; align-items: center; height: 32px; font-size: 12px; font-weight: 600; color: {resolve(v, 'az-on-primary')}; background: {resolve(v, 'az-primary-fill')}; padding: 0 14px; border-radius: 7px">Collect snapshot</span>
          <span style="display: inline-flex; align-items: center; height: 32px; font-size: 12px; font-weight: 500; border: 1px solid {resolve(v, 'az-border')}; background: {surface}; padding: 0 12px; border-radius: 7px">Open data</span>
          <span style="font-size: 12px; font-weight: 500; color: {resolve(v, 'az-primary')}">View inventory →</span>
          <span style="display: inline-flex; align-items: center; height: 32px; font-size: 12px; border: 1px solid {resolve(v, 'az-border')}; background: {surface}; padding: 0 10px; border-radius: 7px; gap: 8px; color: {resolve(v, 'az-text-secondary')}">Search resources, types, groups, tags… <span style="font-size: 11px; color: {resolve(v, 'az-text-muted')}; border: 1px solid {resolve(v, 'az-border')}; border-radius: 5px; padding: 1px 6px; background: {resolve(v, 'az-surface-2')}">⌘K</span></span>
          <span style="display: inline-flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 7px; background: {resolve(v, 'az-selected')}; font-size: 12px"><span style="width: 16px; height: 16px; border-radius: 4px; background: {resolve(v, 'az-resource')}"></span>selected row</span>
          <span class="mono" style="font-size: 11px; background: {resolve(v, 'az-surface-3')}; border: 1px solid {resolve(v, 'az-border-subtle')}; padding: 3px 8px; border-radius: 5px">environment=prod</span>
          <span style="display: inline-flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; color: {resolve(v, 'severity-high')}"><span style="width: 8px; height: 8px; border-radius: 2px; background: {resolve(v, 'severity-high')}"></span>High · 93</span>
        </div>"""

    return f"""<!doctype html>
<!-- @generated by docs/reference/tools/build_design_sheet.py — do not edit by
     hand. Colour values come from desktop/src/styles.css; the prose lives in
     the generator. Re-run it after changing either. -->
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>azdocs · Design language</title>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400..700&amp;display=swap">
  <style>
    body {{ margin: 0; font-family: Inter, "SF Pro Text", "Segoe UI", system-ui, sans-serif; color: {L('az-text-primary')}; background: {L('az-canvas')}; }}
    .page {{ max-width: 1440px; margin: 0 auto; padding: 28px 56px 40px; box-sizing: border-box; display: flex; flex-direction: column; gap: 20px; }}
    a {{ color: {L('az-primary')}; }} a:hover {{ color: {L('az-primary-hover')}; }}
    .mono {{ font-family: ui-monospace, "SF Mono", "Cascadia Code", Menlo, Consolas, monospace; }}
    .sec-title {{ font-size: 18px; font-weight: 650; letter-spacing: -0.01em; margin: 0; }}
    .sec-note {{ font-size: 12.5px; color: {L('az-text-secondary')}; }}
    .swatch-row {{ display: flex; align-items: center; gap: 10px; padding: 5px 0; border-bottom: 1px solid {L('az-border-subtle')}; }}
    .swatch {{ width: 30px; height: 20px; border-radius: 5px; border: 1px solid {L('az-border')}; flex-shrink: 0; }}
    .sw-name {{ font-size: 12px; font-weight: 600; width: 112px; }}
    .sw-hex {{ font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 11px; color: {L('az-text-secondary')}; width: 66px; }}
    .sw-role {{ font-size: 11.5px; color: {L('az-text-secondary')}; flex: 1; }}
    .dk .swatch-row {{ border-bottom: 1px solid {D('az-border-subtle')}; }}
    .dk .swatch {{ border: 1px solid {D('az-border')}; }}
    .dk .sw-name {{ color: {D('az-text-primary')}; }}
    .dk .sw-hex {{ color: {D('az-text-secondary')}; }}
    .dk .sw-role {{ color: {D('az-text-secondary')}; }}
    .anno {{ font-family: ui-monospace, "SF Mono", Menlo, monospace; font-size: 10.5px; color: {L('az-text-secondary')}; }}
    .comp-label {{ font-size: 11px; font-weight: 600; letter-spacing: 0.08em; text-transform: uppercase; color: {L('az-text-muted')}; }}
    .card {{ border: 1px solid {L('az-border')}; border-radius: 10px; background: {L('az-surface-1')}; box-shadow: 0 1px 2px rgb(19 44 69 / 4%), 0 6px 18px rgb(19 44 69 / 3.5%); overflow: hidden; }}
    .columns {{ display: grid; grid-template-columns: 1.1fr 1fr; gap: 44px; }}
    @media (max-width: 1100px) {{ .columns {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
<div class="page">

  <header style="display: flex; align-items: baseline; gap: 18px; border-bottom: 1px solid {L('az-border')}; padding-bottom: 14px; flex-wrap: wrap">
    <span style="font-size: 26px; font-weight: 650; letter-spacing: -0.02em">azdocs · Design language</span>
    <span style="flex: 1"></span>
    <span style="font-size: 12.5px; color: {L('az-text-secondary')}">Dark navy application frame + cool neutral workspace + restrained Azure blue + Inter + subtle depth + highly structured technical data. Tokens: <span class="mono" style="font-size: 11px">desktop/src/styles.css</span> · prose: <a href="design.md">design.md</a></span>
  </header>

  <div class="columns">

    <!-- LEFT: palette -->
    <div style="display: flex; flex-direction: column; gap: 14px">
      <div style="display: flex; align-items: baseline; gap: 12px"><h2 class="sec-title">Palette</h2><span class="sec-note">one set of roles, two modes — dark re-tunes the relationships rather than inverting them</span></div>
      <div class="card" style="display: grid; grid-template-columns: 1fr 1fr; gap: 0">
        <div style="padding: 14px 18px; background: {L('az-surface-1')}">
          <div class="comp-label" style="margin-bottom: 8px">Light · default</div>
{swatch_rows(light, "light")}
        </div>
        <div class="dk" style="padding: 14px 18px; background: {D('az-canvas')}">
          <div class="comp-label" style="margin-bottom: 8px; color: {D('az-text-muted')}">Dark · matte, layered, quiet</div>
{swatch_rows(dark, "dark")}
        </div>
      </div>

      <div>
        <div class="comp-label" style="margin-bottom: 8px">Navigation frame · navy in both modes</div>
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px">
          <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 7px; padding: 8px; border-radius: 10px; background: {L('az-sidebar')}">
{sidebar_tiles(light)}
          </div>
          <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 7px; padding: 8px; border-radius: 10px; background: {D('az-sidebar')}">
{sidebar_tiles(dark)}
          </div>
        </div>
      </div>

      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 24px">
        <div>
          <div class="comp-label" style="margin-bottom: 8px">Semantic accents · one meaning each</div>
          <div style="display: flex; flex-direction: column; gap: 6px">
{accent_tiles(light)}
          </div>
          <div class="sec-note" style="margin-top: 8px">KPI icons sit in 32px containers tinted with their accent at 14% in light and as a low-opacity wash in dark. Purple never becomes a navigation colour on another screen.</div>
        </div>
        <div>
          <div class="comp-label" style="margin-bottom: 8px">Severity · icon + colour + word, never colour alone</div>
          <div style="display: flex; flex-direction: column; gap: 5px">
{severity_rows}
          </div>
          <div class="sec-note" style="margin-top: 8px">Counts always exist in text beside a chart. Rows referenced by open findings carry a dagger (<span style="color: {L('az-danger')}; font-weight: 600">†</span>) in every table.</div>
        </div>
      </div>

      <div>
        <div class="comp-label" style="margin-bottom: 8px">Chart categories · fixed order, CVD-validated both modes</div>
        <div style="display: flex; gap: 6px; margin-bottom: 6px">
{cat_bars}
        </div>
        <div style="display: flex; gap: 6px" class="anno">
          {cat_labels}
        </div>
        <div class="sec-note" style="margin-top: 6px">The top slots of the app's <span class="mono" style="font-size: 11px">category_color()</span> set, never cycled. Charts fold the tail into Other; bars are always direct-labelled. *storage deepens to <span class="mono" style="font-size: 11px">{dark_storage}</span> in dark.</div>
      </div>

      <div style="display: flex; align-items: baseline; gap: 12px; margin-top: 4px"><h2 class="sec-title">Mark</h2><span class="sec-note">a rounded topology A — three nodes and the connectors between them</span></div>
      <div style="display: grid; grid-template-columns: repeat(4, 1fr); gap: 14px">
{mark_tiles}
      </div>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px">
{lockup_tiles}
      </div>
      <div class="sec-note">The reversed mark sits on the navy frame in both modes at 30px. The node diameter is the clear-space unit <span class="mono" style="font-size: 11px">x</span> — keep at least one <span class="mono" style="font-size: 11px">x</span> clear on every side, never crop the mark. Full rules: <a href="../marks/USAGE.md">marks/USAGE.md</a>. Native app icons are derived from the light app icon by <span class="mono" style="font-size: 11px">pnpm run icons</span>, not drawn by hand.</div>
    </div>

    <!-- RIGHT: type + structure -->
    <div style="display: flex; flex-direction: column; gap: 14px; border-left: 1px solid {L('az-border')}; padding-left: 44px">
      <div style="display: flex; align-items: baseline; gap: 12px"><h2 class="sec-title">Type</h2><span class="sec-note">Inter Variable, bundled — one face for the whole interface</span></div>
      <div style="display: flex; flex-direction: column">
{scale}
      </div>
      <div class="sec-note">Nothing below 11px at the default text scale, off a 12px rem base the reader can step with ⌘/Ctrl +, − and 0. Hierarchy comes from size and weight first, then the three text levels. Uppercase is reserved for compact metadata labels (TENANT, SNAPSHOT), never navigation.</div>

      <div style="display: flex; align-items: baseline; gap: 12px; margin-top: 4px"><h2 class="sec-title">Structure</h2><span class="sec-note">layers over rules, borders over shadows in dark</span></div>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; font-size: 12px; color: {L('az-text-primary')}">
        <div>
          <div style="border-top: 1px solid {L('az-border')}; padding-top: 5px; margin-bottom: 8px">1px border — cards, controls, shell boundaries</div>
          <div style="border-top: 1px solid {L('az-border-subtle')}; padding-top: 5px; margin-bottom: 8px">1px subtle border — table rows, list separators</div>
          <div class="card" style="padding: 10px 12px; margin-top: 10px">Card — surface-1, 1px border, 10px radius, a whisper of shadow in light and none in dark</div>
        </div>
        <div>
          <div style="margin-bottom: 6px">Spacing: <span class="mono">4 · 8 · 12 · 16 · 20 · 24 · 32 · 40 · 48</span> (a 4px base, written in rem so the text-size step scales it)</div>
          <div style="margin-bottom: 6px">Radii: <span class="mono">5px</span> badges &amp; chips · <span class="mono">7px</span> controls · <span class="mono">8px</span> menus · <span class="mono">10px</span> cards · <span class="mono">12px</span> large panels · <span class="mono">14px</span> dialogs. Pills only for true statuses, filters and tags.</div>
          <div style="margin-bottom: 6px">Frame: sidebar <span class="mono">208px</span> (rail <span class="mono">64px</span>, <span class="mono">80px</span> under the macOS overlay title bar) · command bar <span class="mono">56px</span> · workspace padding <span class="mono">24 × 20</span> · content stops stretching past <span class="mono">~1840px</span>.</div>
          <div>Controls are <span class="mono">32px</span> tall (28 compact, 36 prominent). Motion <span class="mono">120–180ms</span>, <span class="mono">cubic-bezier(.2,.8,.2,1)</span>; never animate KPI values or charts on page entry.</div>
        </div>
      </div>

      <div style="display: flex; align-items: baseline; gap: 12px; margin-top: 4px"><h2 class="sec-title">Components</h2><span class="sec-note">same anatomy in both modes</span></div>
      <div class="card">
{components(light, "light")}
{components(dark, "dark")}
      </div>
      <div class="sec-note">Three button levels — primary (blue fill, white label; one per surface), secondary (bordered neutral), tertiary (text). Selection anywhere is the quiet <span class="mono" style="font-size: 11px">--az-selected</span> fill; the sidebar's selected row is a soft blue emphasis, never a thick coloured bar. Theme is a tri-state in Settings — <span style="font-weight: 600">System</span> (default) · Light · Dark — and every chart, SVG and the topology canvas reads the same tokens.</div>
    </div>
  </div>

  <footer style="display: flex; align-items: center; gap: 14px; border-top: 1px solid {L('az-border')}; padding-top: 10px; font-size: 11.5px; color: {L('az-text-secondary')}; flex-wrap: wrap">
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
