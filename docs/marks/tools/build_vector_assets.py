#!/usr/bin/env python3
"""Build the azdocs production SVG family from exact vector geometry."""

from __future__ import annotations

from math import copysign, cos, pi, sin
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
DEEP_BLUE = "#0754bd"
WHITE = "#ffffff"


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
    <linearGradient id="mark-blue" x1="0.18" y1="0" x2="0.72" y2="1">
      <stop offset="0" stop-color="#168fe5"/>
      <stop offset="0.52" stop-color="#0b84dc"/>
      <stop offset="1" stop-color="#0078d4"/>
    </linearGradient>
    """.strip()


def topology_paths(colour: str) -> str:
    return f"""
    <g fill="{colour}" stroke="{colour}">
      <path d="M256 198 V330 L156 382 M256 330 L356 382"
        fill="none" stroke-width="26" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="256" cy="198" r="40" stroke="none"/>
      <circle cx="156" cy="382" r="40" stroke="none"/>
      <circle cx="356" cy="382" r="40" stroke="none"/>
    </g>
    """.strip()


def colour_mark(*, topology: str = WHITE) -> str:
    return f"""
  <g>
    <path d="M232 58 C242 38 270 38 280 58 L480 414
      C495 441 476 472 444 472 H68 C36 472 17 441 32 414 Z"
      fill="url(#mark-blue)"/>
    {topology_paths(topology)}
  </g>
    """.strip()


def mono_definitions() -> str:
    return """
    <mask id="mark-separation" maskUnits="userSpaceOnUse" x="0" y="0" width="512" height="512">
      <rect width="512" height="512" fill="white"/>
      <path d="M256 198 V330 L156 382 M256 330 L356 382"
        fill="none" stroke="black" stroke-width="28" stroke-linecap="round" stroke-linejoin="round"/>
      <circle cx="256" cy="198" r="42" fill="black"/>
      <circle cx="156" cy="382" r="42" fill="black"/>
      <circle cx="356" cy="382" r="42" fill="black"/>
    </mask>
    """.strip()


def mono_mark(colour: str) -> str:
    return f"""
  <path d="M232 58 C242 38 270 38 280 58 L480 414
    C495 441 476 472 444 472 H68 C36 472 17 441 32 414 Z"
    fill="{colour}" mask="url(#mark-separation)"/>
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
    for name, topology in (
        ("azdocs-mark-primary", WHITE),
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
                body=colour_mark(topology=topology),
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
    # The original superellipse keeps a consistent Dock silhouette. The 40-unit
    # outer margin belongs to the native icon canvas, not to the standalone A.
    points = []
    for step in range(256):
        angle = 2 * pi * step / 256
        x = 256 + 216 * copysign(abs(cos(angle)) ** 0.5, cos(angle))
        y = 256 + 216 * copysign(abs(sin(angle)) ** 0.5, sin(angle))
        points.append(f"{x:.3f},{y:.3f}")
    tile_path = "M" + " L".join(points) + " Z"

    for name, topology, background in (
        ("azdocs-app-icon-light", WHITE, PAPER),
        ("azdocs-app-icon-dark", PAPER, CHARCOAL),
    ):
        write_asset(
            name,
            svg_document(
                title=name.replace("-", " "),
                view_box="0 0 512 512",
                width=2048,
                height=2048,
                definitions=colour_definitions(),
                body=(
                    f'<path d="{tile_path}" fill="{background}"/>\n'
                    # Lift the triangular mark slightly to balance its wider base.
                    '<g transform="translate(71.68 58) scale(0.72)">\n'
                    f"{colour_mark(topology=topology)}\n</g>"
                ),
            ),
        )


def build_lockups() -> None:
    for name, topology, word_colour in (
        ("azdocs-lockup-primary", WHITE, DEEP_BLUE),
        ("azdocs-lockup-reversed", PAPER, PAPER),
    ):
        body = (
            f"  {wordmark_paths(text='zdocs', colour=word_colour, x=206, baseline=201, font_size=150, tracking=-12)}\n"
            f'<g transform="translate(12 27) scale(0.44)">{colour_mark(topology=topology)}</g>'
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
    for name, background, rule, topology in (
        ("azdocs-background-light", PAPER, HAIRLINE, WHITE),
        ("azdocs-background-dark", CHARCOAL, DARK_HAIRLINE, PAPER),
    ):
        body = f"""
  <rect width="1920" height="1080" fill="{background}"/>
  <g transform="translate(1110 -55) scale(2.25)" opacity="0.075">
    {colour_mark(topology=topology)}
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
