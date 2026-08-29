#!/usr/bin/env python3
"""Build the azdocs production SVG family from exact vector geometry."""

from __future__ import annotations

from pathlib import Path
from xml.sax.saxutils import escape

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont


ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
SVG_DIR = ROOT / "assets"
FONT_PATH = REPO / "data/fonts/IBMPlexSans-Bold.ttf"

PAPER = "#faf8f4"
INK = "#1c2430"
CHARCOAL = "#14181d"
HAIRLINE = "#d8d2c6"
DARK_HAIRLINE = "#3a4048"
GRAPH_INK = "#0b2d4a"
NODE_BLUE = "#159cf0"
DEEP_BLUE = "#0754bd"


def svg_document(
    *,
    title: str,
    view_box: str,
    width: int,
    height: int,
    body: str,
    definitions: str = "",
) -> str:
    defs = f"<defs>{definitions}</defs>" if definitions else ""
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<svg xmlns="http://www.w3.org/2000/svg" role="img" '
        f'viewBox="{view_box}" width="{width}" height="{height}">\n'
        f"  <title>{escape(title)}</title>\n"
        f"  {defs}\n"
        f"{body}\n"
        "</svg>\n"
    )


def colour_definitions() -> str:
    return """
    <linearGradient id="ribbon-deep" x1="0" y1="1" x2="0.72" y2="0">
      <stop offset="0" stop-color="#0754bd"/>
      <stop offset="0.56" stop-color="#0b68d5"/>
      <stop offset="1" stop-color="#1479e6"/>
    </linearGradient>
    <linearGradient id="ribbon-light" x1="0.12" y1="0" x2="0.88" y2="1">
      <stop offset="0" stop-color="#39c8f5"/>
      <stop offset="0.48" stop-color="#21b1ed"/>
      <stop offset="1" stop-color="#168fe2"/>
    </linearGradient>
    <linearGradient id="node-blue" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#27b5f5"/>
      <stop offset="1" stop-color="#0b86e7"/>
    </linearGradient>
    """.strip()


def graph_paths(*, connector: str, shadows: bool) -> str:
    shadow = ""
    if shadows:
        shadow = """
    <g transform="translate(0 7)" fill="#061828" stroke="#061828" opacity="0.22">
      <path d="M256 253 L184 352 L328 352 Z" fill="none" stroke-width="22" stroke-linejoin="round"/>
      <circle cx="256" cy="253" r="36" stroke="none"/>
      <circle cx="184" cy="352" r="36" stroke="none"/>
      <circle cx="328" cy="352" r="36" stroke="none"/>
    </g>
        """.strip()
    return f"""
    {shadow}
    <g>
      <path d="M256 253 L184 352 L328 352 Z" fill="none" stroke="{connector}" stroke-width="20" stroke-linejoin="round"/>
      <g fill="url(#node-blue)" stroke="#52c9f8" stroke-width="2">
        <circle cx="256" cy="253" r="34"/>
        <circle cx="184" cy="352" r="34"/>
        <circle cx="328" cy="352" r="34"/>
      </g>
    </g>
    """.strip()


def colour_mark(*, connector: str, shadows: bool = True) -> str:
    return f"""
  <g>
    <path d="M52 428 L203 84 L247.68 190 L156 428 Z" fill="url(#ribbon-deep)"/>
    <path d="M203 84 H300 L460 428 H348 Z" fill="url(#ribbon-light)"/>
    <path d="M300 84 L338 166 H318 Q300 166 300 148 Z" fill="#f3eee5"/>
    {graph_paths(connector=connector, shadows=shadows)}
  </g>
    """.strip()


def mono_definitions() -> str:
    return """
    <mask id="mark-separation" maskUnits="userSpaceOnUse" x="0" y="0" width="512" height="512">
      <rect width="512" height="512" fill="white"/>
      <path d="M256 253 L184 352 L328 352 Z" fill="none" stroke="black" stroke-width="30" stroke-linejoin="round"/>
      <circle cx="256" cy="253" r="41" fill="black"/>
      <circle cx="184" cy="352" r="41" fill="black"/>
      <circle cx="328" cy="352" r="41" fill="black"/>
      <path d="M300 84 V148 Q300 166 318 166 H338" fill="none" stroke="black" stroke-width="6"/>
    </mask>
    """.strip()


def mono_mark(colour: str) -> str:
    return f"""
  <g fill="{colour}" mask="url(#mark-separation)">
    <path d="M52 428 L203 84 L247.68 190 L156 428 Z"/>
    <path d="M203 84 H300 L460 428 H348 Z"/>
  </g>
  <g fill="{colour}" stroke="{colour}">
    <path d="M256 253 L184 352 L328 352 Z" fill="none" stroke-width="20" stroke-linejoin="round"/>
    <circle cx="256" cy="253" r="34" stroke="none"/>
    <circle cx="184" cy="352" r="34" stroke="none"/>
    <circle cx="328" cy="352" r="34" stroke="none"/>
  </g>
    """.strip()


def wordmark_paths(
    *,
    text: str,
    colour: str,
    x: float,
    baseline: float,
    font_size: float,
    tracking: float = 0,
    first_colour: str | None = None,
) -> str:
    font = TTFont(FONT_PATH)
    glyph_set = font.getGlyphSet()
    cmap = font.getBestCmap()
    metrics = font["hmtx"].metrics
    units_per_em = font["head"].unitsPerEm
    scale = font_size / units_per_em
    cursor = 0
    paths: list[str] = []
    for index, character in enumerate(text):
        glyph_name = cmap[ord(character)]
        pen = SVGPathPen(glyph_set)
        glyph_set[glyph_name].draw(pen)
        command = pen.getCommands()
        glyph_colour = first_colour if index == 0 and first_colour else colour
        paths.append(
            f'<path d="{command}" fill="{glyph_colour}" stroke="{glyph_colour}" '
            f'stroke-width="24" stroke-linejoin="round" paint-order="stroke fill" '
            f'transform="translate({cursor:.3f} 0)"/>'
        )
        cursor += metrics[glyph_name][0] + tracking
    glyphs = "".join(paths)
    return (
        f'<g transform="translate({x:g} {baseline:g}) scale({scale:.6f} '
        f'-{scale:.6f})">{glyphs}</g>'
    )


def write_asset(name: str, content: str) -> None:
    (SVG_DIR / f"{name}.svg").write_text(content, encoding="utf-8")


def build_mark_assets() -> None:
    for name, connector in (
        ("azdocs-mark-primary", GRAPH_INK),
        ("azdocs-mark-reversed", PAPER),
    ):
        write_asset(
            name,
            svg_document(
                title=name.replace("-", " "),
                view_box="0 0 512 512",
                width=2048,
                height=2048,
                definitions=colour_definitions(),
                body=colour_mark(connector=connector),
            ),
        )

    for name, colour in (
        ("azdocs-mark-mono-ink", INK),
        ("azdocs-mark-mono-paper", PAPER),
    ):
        write_asset(
            name,
            svg_document(
                title=name.replace("-", " "),
                view_box="0 0 512 512",
                width=2048,
                height=2048,
                definitions=mono_definitions(),
                body=mono_mark(colour),
            ),
        )


def build_app_icons() -> None:
    for name, background, border, connector in (
        ("azdocs-app-icon-light", PAPER, HAIRLINE, GRAPH_INK),
        ("azdocs-app-icon-dark", CHARCOAL, DARK_HAIRLINE, PAPER),
    ):
        body = f"""
  <rect width="512" height="512" rx="104" fill="{background}"/>
  <rect x="12" y="12" width="488" height="488" rx="96" fill="none" stroke="{border}" stroke-width="4"/>
  {colour_mark(connector=connector)}
        """.strip()
        write_asset(
            name,
            svg_document(
                title=name.replace("-", " "),
                view_box="0 0 512 512",
                width=2048,
                height=2048,
                definitions=colour_definitions(),
                body=body,
            ),
        )


def build_lockups() -> None:
    for name, connector, word_colour in (
        ("azdocs-lockup-primary", GRAPH_INK, DEEP_BLUE),
        ("azdocs-lockup-reversed", PAPER, PAPER),
    ):
        body = (
            f"  {wordmark_paths(text='zdocs', colour=word_colour, x=190, baseline=201, font_size=150, tracking=-12, first_colour='#168fe2')}\n"
            f'<g transform="translate(12 27) scale(0.44)">{colour_mark(connector=connector)}</g>'
        )
        write_asset(
            name,
            svg_document(
                title=name.replace("-", " "),
                view_box="0 0 640 256",
                width=2560,
                height=1024,
                definitions=colour_definitions(),
                body=body,
            ),
        )


def build_backgrounds() -> None:
    for name, background, rule, connector in (
        ("azdocs-background-light", PAPER, HAIRLINE, GRAPH_INK),
        ("azdocs-background-dark", CHARCOAL, DARK_HAIRLINE, PAPER),
    ):
        body = f"""
  <rect width="1920" height="1080" fill="{background}"/>
  <g transform="translate(1110 -55) scale(2.25)" opacity="0.075">
    {colour_mark(connector=connector, shadows=False)}
  </g>
  <path d="M0 890 H1050 V770 H1260" fill="none" stroke="{rule}" stroke-width="3"/>
  <circle cx="1050" cy="890" r="8" fill="{rule}"/>
  <circle cx="1260" cy="770" r="8" fill="{rule}"/>
        """.strip()
        write_asset(
            name,
            svg_document(
                title=name.replace("-", " "),
                view_box="0 0 1920 1080",
                width=3840,
                height=2160,
                definitions=colour_definitions(),
                body=body,
            ),
        )


def main() -> None:
    SVG_DIR.mkdir(parents=True, exist_ok=True)
    build_mark_assets()
    build_app_icons()
    build_lockups()
    build_backgrounds()


if __name__ == "__main__":
    main()
