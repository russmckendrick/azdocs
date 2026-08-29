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
3. Exactly three topology nodes joined as a triangle and drawn over the A.

Do not move the fold down a leg, add nodes, introduce a crossbar, or substitute
the official Azure artwork. Both inner and outer leg boundaries are straight
single segments. The deep-blue ribbon continues from the left foot to the
crown; the light-blue ribbon continues from the crown to the right foot. The
lower nodes deliberately overlap the inner ribbon edges so the graph reads as
the foremost layer.

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
| Deep ribbon | `#0754bd` → `#1479e6` | Left foot to crown |
| Light ribbon | `#39c8f5` → `#168fe2` | Crown to right foot |
| Node blue | `#27b5f5` → `#0b86e7` | Foreground topology nodes |
| Graph ink | `#0b2d4a` | Light-surface connector bars |
| Ink | `#1c2430` | Neutral supporting text |
| Paper | `#faf8f4` | Light surface and reversed artwork |
| Evidence paper | `#f3eee5` | Page fold and secondary paper |
| Warm charcoal | `#14181d` | Dark icon/background surface |
| Hairline | `#d8d2c6` | Fold and light tile boundary |
| Dark hairline | `#3a4048` | Dark tile boundary |

These values align with the desktop's Field Report tokens. Gradients are
restricted to the two ribbons and the three nodes. A short, neutral shadow is
allowed only beneath the topology to separate its foreground layer; do not add
a glow, bevel, long shadow or shadow around the A itself. Colour remains
structural: the blues identify the folded Azure-estate planes, while ink/paper
separates the topology from them.

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

The lockup uses the folded-topology mark itself as the initial **A**, followed
closely by `zdocs` in IBM Plex Sans Bold converted to SVG outlines. The
cyan `z` continues the colour and direction of the A's right ribbon; `docs`
continues in deep ribbon blue on light surfaces and warm paper on dark surfaces.
The `z` passes behind the A's right foot so the mark remains the foremost
layer. The supplied outlines include a restrained same-colour optical expansion
to balance the heavy A. This is one integrated word shape, not a symbol beside
a repeated `azdocs`. Use the supplied lockup rather than recreating the text
with a local font. Do not change its spacing or colour sequence, or place a
tagline between the mark and `zdocs`.

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
- replace the straight ribbon boundaries with an automatic raster trace;
- remove or relocate the top fold;
- add cloud, shield, checkmark or magnifying-glass symbols;
- use more or fewer than three topology nodes;
- stretch, shear, rotate or add perspective;
- change individual elements to severity or category colours;
- typeset a replacement wordmark;
- use the generated PNG concept boards as final artwork.

## Export and implementation

Keep the SVG `viewBox` when exporting. The supplied PNGs are already rendered
at 2048×2048 for marks and icons, 2560×1024 for lockups, and 3840×2160 for
backgrounds. Generate platform PNG/ICO/ICNS assets from the light or dark
app-icon SVG at the final required sizes; do not upscale from a small raster.
Keep transparency for standalone marks and lockups.

The current SVGs are the design sources in `docs/marks/`; desktop consumers
should reference or derive from them rather than maintaining separate artwork.

The desktop masthead and browser favicons load their light/dark SVGs directly
from `docs/marks/assets/`. Native application bundles require platform-specific
formats. Tauri's dev and package builds regenerate those derivatives
automatically from the same app-icon SVG. To refresh them without starting a
build, run:

```sh
cd desktop
npm run icons
```

At the desktop masthead's constrained height, use the standalone mark followed
by a bold live-text `zdocs`. This preserves the A as the initial letter while
keeping the small signature crisp. Reserve the outlined integrated lockup for
larger headers, covers and marketing placements.

## Brand distinction

This is an original azdocs identity intended to evoke Azure estate work without
presenting azdocs as a Microsoft product. Do not combine it with the Microsoft
wordmark or describe it as an official Azure mark. Arrange an appropriate brand
or trademark review before a public commercial release.
