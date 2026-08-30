# Mark usage

## Choosing an asset

Use the horizontal lockup when the audience needs the product name. Use the
standalone mark when azdocs is already named by the surrounding interface or
document. The app icons have transparent outer corners so each platform can
apply its own launch treatment; do not place them inside a second badge.

| Context | Preferred asset |
|---|---|
| Desktop masthead, website header | `azdocs-lockup-primary.svg` |
| Dark masthead or footer | `azdocs-lockup-reversed.svg` |
| Application launcher, avatar | Matching `azdocs-app-icon-*.svg` mark |
| Favicon or compact navigation | Matching standalone mark |
| PDF/DOCX cover | `azdocs-mark-primary.svg` on paper; `azdocs-mark-mono-paper.svg` on a dark cover block |
| Slide or social background | Matching `azdocs-background-*.svg` motif |
| Single-colour print or fabrication | Matching `azdocs-mark-mono-*.svg` mark |

## Construction

The simplified identity has three load-bearing parts:

1. One broad, symmetric, rounded triangular **A** silhouette.
2. Exactly three equal circular topology nodes in white negative space.
3. One Y-shaped connector joining the three nodes at the optical centre.

Do not add facets, a page fold, separate legs, extra nodes or a conventional
crossbar, and do not substitute the official Azure artwork. The topology is the
counter of the A rather than an independent foreground badge.

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
| Mark blue | `#168fe5` → `#0078d4` | Rounded A silhouette |
| Topology | `#ffffff` | Nodes and connector on light launch surfaces |
| Reversed topology | `#faf8f4` | Nodes and connector on dark surfaces |
| Ink | `#1c2430` | Neutral supporting text |
| Paper | `#faf8f4` | Light surface and reversed artwork |
| Warm charcoal | `#14181d` | Dark icon/background surface |
| Hairline | `#d8d2c6` | Light background construction rules |
| Dark hairline | `#3a4048` | Dark background construction rules |

These values align with the desktop's Field Report tokens. The only permitted
gradient is the restrained blue transition across the outer silhouette. Do not
add a glow, bevel, texture or drop shadow; the white topology must remain flat
and optically centred.

## Backgrounds

- Use the primary artwork on `#faf8f4`, white, or similarly quiet light
  surfaces.
- Use reversed artwork on `#14181d` or similarly quiet dark surfaces.
- Do not place the colour mark on saturated blue; use an appropriate
  monochrome version instead.
- Avoid photography or diagrams behind the mark. If unavoidable, place it on a
  plain paper or charcoal field with the required clear space.
- Keep titles and body copy out of the oversized mark crop in the supplied
  backgrounds. The left two-thirds are the intended content area.

## Wordmark

The lockup uses the topology mark itself as the initial **A**, followed closely
by `zdocs` in IBM Plex Sans Bold converted to SVG outlines. The text is deep
blue on light surfaces and warm paper on dark surfaces. This is one word shape,
not a symbol beside a repeated `azdocs`. Use the supplied lockup rather than
recreating the outlined text or placing a tagline between the mark and `zdocs`.

When nearby text already says “azdocs”, use the standalone mark and avoid
repeating the wordmark.

## Accessibility

- If visible text already names azdocs, treat the mark as decorative with an
  empty alt value.
- If the mark is the only product identifier, use `alt="azdocs"`.
- Do not rely on the blue gradient alone to communicate meaning. The silhouette
  and node geometry must remain present in monochrome.
- Check the final raster at its real display size, especially at 24–32 px.

## Misuse

Do not:

- redraw the outer shape as the official Azure A;
- replace the exact rounded geometry with an automatic raster trace;
- add facets, folds, bevels or shadows;
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

The command compares the source SVG hash with the generated native set before
running Tauri's converter. Use `npm run icons -- --force` only when the icon
toolchain itself changes and the same SVG must be regenerated.

At the desktop masthead's constrained height, use the standalone mark followed
by a bold live-text `zdocs`. This preserves the A as the initial letter while
keeping the small signature crisp. Reserve the outlined integrated lockup for
larger headers, covers and marketing placements.

## Brand distinction

This is an original azdocs identity intended to evoke Azure estate work without
presenting azdocs as a Microsoft product. Do not combine it with the Microsoft
wordmark or describe it as an official Azure mark. Arrange an appropriate brand
or trademark review before a public commercial release.
