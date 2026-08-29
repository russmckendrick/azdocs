# Mark usage

## Choosing an asset

Use the horizontal lockup when the audience needs the product name. Use the
standalone mark when azdocs is already named by the surrounding interface or
document. The app-icon tiles own their backgrounds; do not put them inside a
second tile, circle or badge.

| Context | Preferred asset |
|---|---|
| Desktop masthead, website header | `azdocs-lockup-primary.svg` |
| Dark masthead or footer | `azdocs-lockup-reversed.svg` |
| Application launcher, avatar | Matching `azdocs-app-icon-*.svg` tile |
| Favicon or compact navigation | Matching standalone mark |
| PDF/DOCX cover, slide, social background | Matching `azdocs-background-*.svg` motif |
| Single-colour print or fabrication | Matching `azdocs-mark-mono-*.svg` mark |

## Construction

The identity has three load-bearing parts:

1. A broad angular **A** with enough weight to survive icon sizing.
2. A small page fold cut into the top-right of the crown.
3. Exactly three topology nodes joined as a triangle in the counter.

Do not move the fold down a leg, add nodes, introduce a crossbar, or substitute
the official Azure artwork. The right leg may use the darker accent facet in
colour versions; monochrome versions collapse the letter to one plane.

## Clear space and minimum size

The node diameter is the clear-space unit, **x**. Keep at least one x clear on
every side of a standalone mark or lockup. The app-icon SVG already contains
its required internal padding.

- Standalone mark: at least **24 CSS px** high.
- App icon: at least **32 CSS px** square.
- Horizontal lockup: at least **140 CSS px** wide.
- Below these sizes, inspect the target raster rather than assuming the three
  connectors will survive hinting.

Never crop the mark itself. Cropping is allowed only in the supplied background
motifs, where it is an intentional part of the composition.

## Colour

| Role | Value | Source |
|---|---|---|
| Azure blue | `#0078d4` | Primary plane and topology nodes |
| azdocs accent | `#0b5da8` | Folded/right plane |
| Dark-mode node blue | `#5aa7e8` | Topology nodes on charcoal |
| Ink | `#1c2430` | Light-surface connectors and wordmark |
| Paper | `#faf8f4` | Light surface and reversed artwork |
| Evidence paper | `#f3efe7` | Page fold and secondary paper |
| Warm charcoal | `#14181d` | Dark icon/background surface |
| Hairline | `#d8d2c6` | Fold and light tile boundary |
| Dark hairline | `#3a4048` | Dark tile boundary |

These values align with the desktop's Field Report tokens. Do not add a
gradient, glow, bevel or drop shadow. Colour remains structural: blue identifies
the folded Azure-estate plane; ink/paper separates the topology inside it.

## Backgrounds

- Use the primary artwork on `#faf8f4`, white, or similarly quiet light
  surfaces.
- Use reversed artwork on `#14181d` or similarly quiet dark surfaces.
- Do not place the colour mark on saturated blue; use an appropriate
  monochrome version instead.
- Avoid photography or diagrams behind the mark. If unavoidable, place it on
  one of the supplied app-icon tiles.
- Keep titles and body copy out of the oversized mark crop in the supplied
  backgrounds. The left two-thirds are the intended content area.

## Wordmark

The lockup wordmark is IBM Plex Sans SemiBold converted to SVG outlines. Use
the supplied lockup rather than recreating the text with a local font. Do not
change its spacing, independently recolour letters, or place a tagline between
the symbol and wordmark.

When nearby text already says “azdocs”, use the standalone mark and avoid
repeating the wordmark.

## Accessibility

- If visible text already names azdocs, treat the mark as decorative with an
  empty alt value.
- If the mark is the only product identifier, use `alt="azdocs"`.
- Do not rely on the blue facet alone to communicate meaning. The silhouette,
  fold and node geometry must remain present in monochrome.
- Check the final raster at its real display size, especially at 24–32 px.

## Misuse

Do not:

- redraw the outer shape as the official Azure A;
- remove or relocate the top fold;
- add cloud, shield, checkmark or magnifying-glass symbols;
- use more or fewer than three topology nodes;
- stretch, shear, rotate or add perspective;
- change individual elements to severity or category colours;
- typeset a replacement wordmark;
- use the generated PNG concept boards as final artwork.

## Export and implementation

Keep the SVG `viewBox` when exporting. Generate platform PNG/ICO/ICNS assets
from the light or dark app-icon SVG at the final required sizes; do not upscale
from a small raster. Keep transparency for standalone marks and lockups.

The current SVGs are design sources in `docs/marks/`. Replacing the desktop
masthead art or Tauri packaging icons should be a separate change so each target
can be visually checked on light and dark surfaces.

## Brand distinction

This is an original azdocs identity intended to evoke Azure estate work without
presenting azdocs as a Microsoft product. Do not combine it with the Microsoft
wordmark or describe it as an official Azure mark. Arrange an appropriate brand
or trademark review before a public commercial release.
