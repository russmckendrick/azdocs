# Themes

Themes are data. A theme is a TOML file that supplies **values** — colours, a
type scale, and a choice from a closed set of layout strategies. Every emitter
implements each strategy once and branches only on those values, so a new
theme is a new TOML file, never new Rust.

```toml
# azdocs.toml
[branding]
theme = "azure"
```

`azure` is the default when `theme` is omitted. Two themes ship with azdocs;
set a custom theme explicitly when an organisation needs its own document
system. The desktop Exports page previews every available theme, resolved
against your branding colours, and can override the configured one per export.

Built-in themes live in `data/themes/` and are embedded in the binary. Drop
files into `<config dir>/azdocs/themes/` to add your own or replace a built-in
of the same name — the same drop-in pattern as
[`queries.d/`](../usage/queries.md#custom-queries). An unknown theme name is an error
that lists the names that do exist.

| Theme | Look |
|---|---|
| `azure` | The desktop in print: the navy frame as a gradient cover, Azure-blue sans-serif headings, tinted banded tables with zebra rows, tinted summary cards, the desktop's severity and service-family colours, and its full-colour world map. |
| `field-report` | Paper and ink: a warm paper-gradient cover, serif-led hierarchy, hairline tables, quiet evidence fills, and colour reserved for identity and signals. |

Themes drive the PDF, DOCX, HTML report, docs site and XLSX. The Markdown and
CSV outputs are deliberately unstyled.

## Colour expressions

Palette values are expressions over the two `[branding]` colours, so a theme
adapts to the brand instead of hardcoding hex:

| Form | Example |
|---|---|
| Literal | `"#0078d4"` |
| Branding reference | `"$primary"`, `"$accent"` |
| `lighten(colour, 0..1)` | `"lighten($primary, 0.88)"` — blend toward white |
| `darken(colour, 0..1)` | `"darken($primary, 0.2)"` — blend toward black |
| `mix(from, to, 0..1)` | `"mix($primary, #ffffff, 0.9)"` |
| `readable_on(colour)` | `"readable_on($primary)"` — white or near-black, whichever has more contrast |

Colour arguments may themselves be calls, so a theme that derives its primary
keeps the text on it legible: `readable_on(darken($primary, 0.25))`.

`readable_on` is the reason a pale `primary_color` does not produce
white-on-white table headers. Use it for any colour drawn **on top of** a
brand colour.

## Schema

Every key has a default, so a theme file only states what it changes. Unknown
keys are an error, so a typo fails loudly rather than being ignored.

```toml
title = "Acme"            # display name in the desktop picker; defaults to the file stem
description = "One line, shown in the theme listing."

[palette]
primary       = "$primary"
primary_dark  = "darken($primary, 0.18)"
primary_tint  = "lighten($primary, 0.88)"
accent        = "$accent"
accent_tint   = "lighten($accent, 0.85)"
on_primary    = "readable_on($primary)"   # text on a primary fill
band          = "$primary"                # cover band/block
on_band       = "readable_on($primary)"
ink           = "#1a1a2e"
muted         = "#65656f"
rule          = "#d9d9e0"
surface       = "#ffffff"
zebra         = "lighten($primary, 0.95)" # alternating row fill
bar           = "$muted"                  # single-series chart bars
bar_track     = "$zebra"                  # the track behind every bar

# Service-family chart bars, one colour per family.
[palette.series]
network = "$muted"
compute = "$muted"
data = "$muted"
identity = "$muted"
monitoring = "$muted"
integration = "$muted"
other = "$muted"

# The resource-locations world map. Land shades from `land` (north) to
# `land_south`; an empty `marker_ring` draws no halo.
[palette.map]
sea = "$zebra"
land = "$rule"
land_south = "$rule"
coast = "$surface"
grid = "$rule"
marker = "$accent"
marker_ring = ""
leader = "$muted"                          # line from a displaced marker to its region

# One block per severity: high, medium, low, info.
[palette.severity.high]
text = "#a4262c"
fill = "#f8cecc"

[typography]
reference_scale = 0.9     # working text in the technical reference; 8pt floor
serif      = "IBM Plex Serif" # display face for covers, headings and figures
sans       = "IBM Plex Sans"  # working face for PDF + HTML (see Fonts below)
mono       = "IBM Plex Mono"
pdf_use_docx_fonts = true     # use installed Word families in PDF, with bundled fallbacks
docx_serif = "Charter"        # Word resolves these on the reader's machine
docx_sans  = "Arial"
docx_mono  = "Courier New"
base_pt = 11.0
small_pt = 9.0
table_pt = 9.0
table_header_pt = 9.0
title_pt = 32.0
subtitle_pt = 14.0
h1_pt = 20.0
h2_pt = 15.0
h3_pt = 12.0
stat_value_pt = 22.0
stat_label_pt = 9.0
line_height = 1.45

[layout]
reference_table_borders = true  # full table grid in the technical reference
reference_table_inset_pt = 3.0  # compact cell padding in that document
cover = "editorial"       # band | editorial | block
table = "hairline"        # solid-header | hairline | banded
stat  = "bare"            # card | outline | bare
heading_numbering = false
divider_pages = false     # a full page before each chapter
running_header = true
zebra_rows = false
rule_pt = 0.5
radius_pt = 3.0
table_inset_pt = 6.0
cover_band_pt = 0.0
cover_background = ""     # an A4 SVG drawn behind the cover; see Cover artwork
```

### Layout strategies

These three keys are the contract between data and code. Each emitter
implements all of the variants; a theme picks one.

| Key | Variant | Effect |
|---|---|---|
| `cover` | `band` | Colour band across the top, content left-aligned below |
| | `editorial` | Centred type with a hairline rule, no fills |
| | `block` | Colour block with a reversed-out title |
| `table` | `solid-header` | Filled header with reversed text, full grid |
| | `hairline` | Horizontal hairlines only, no header fill |
| | `banded` | Tinted header, horizontal rules only |
| `stat` | `card` | Tinted card with an accent top rule |
| | `outline` | Hairline-outlined box |
| | `bare` | No box: value over label |

## Cover artwork

`cover_background` names an A4 portrait SVG that is drawn full-bleed behind the
cover. It fills the whole sheet behind a `block` or `editorial` cover and the
strip at the top of a `band` cover (cropped from the artwork's head), in place
of the flat `band` fill. The cover type sits on it — `on_band` for block and
band, `ink` for editorial — so the artwork must keep that colour legible.

Built-in artwork lives in `data/themes/backgrounds/`: `azure-gradient.svg`
(navy to Azure, under white type) and `field-report-paper.svg` (a warm paper
gradient, under ink). Drop your own into `<config dir>/azdocs/themes/backgrounds/`;
a file with the same name replaces the built-in.

Artwork can follow the theme and branding colours: a double-braced palette name
is replaced with that resolved colour before rendering. For example,
`stop-color="{{accent}}"` becomes the brand accent. The names are the light
palette fields: `primary`, `primary_dark`, `primary_tint`, `accent`,
`accent_tint`, `on_primary`, `band`, `on_band`, `ink`, `muted`, `rule`,
`surface` and `zebra`. An unknown name, or a missing file, is an error rather
than a silent fallback.

The PDF embeds the SVG as vector artwork. Word has no per-page background, so
the DOCX rasterises it at 120 dpi and anchors it to the page, behind the text,
from the cover's first paragraph. It is deliberately not in a header: Word
greys header content out while you edit, which would wash the cover out.

## Fonts

Azure sets everything in the bundled IBM Plex Sans and Mono in the PDF, and in
Arial and Courier New in Word. Field Report uses Charter Regular for headings, Arial for body text and tables,
and Courier New for identifiers in both native print formats.
`pdf_use_docx_fonts = true` makes the PDF prefer the theme's `docx_*` families.
Branding resolution loads only those installed families, offline, in a stable
order; the PDF embeds the faces it uses. No operating-system font files are
redistributed with azdocs.

Where a requested family is unavailable, PDF falls back to the corresponding
`serif`/`sans`/`mono` theme family. [IBM Plex](https://github.com/IBM/plex) Serif,
Sans and Mono are bundled under the SIL Open Font License 1.1. A fixed snapshot,
theme and font files produce deterministic PDF bytes; changing installed fonts
can change pagination. Set `pdf_use_docx_fonts = false` for bundled-only type.

To use a different typeface in the PDF, point `[branding] font_dir` at a
directory of `.ttf`/`.otf` files and name the family:

```toml
[branding]
font_dir = "fonts/"          # relative to this config file
font_family = "Acme Grotesk"
mono_family = "Acme Mono"
```

**DOCX is different.** OOXML names a font and resolves it on the reader's
machine, and azdocs cannot embed fonts into a `.docx`, so naming a typeface
nobody has installed lands back at a Word default. Charter is installed on
macOS; other machines need that face installed for identical headings.
If your organisation deploys its own typeface, override the three `docx_*`
keys in a user theme. Explicit `font_family`/`mono_family` branding overrides
disable matching so custom PDF font selections still take precedence.

## Native renderer differences

PDF and DOCX receive the same semantic `PrintDocument`, including heading
numbering intent, divider chapters, running headers, captions and severity
roles. Their layout engines still have unavoidable differences:

- Typst embeds the configured fonts; Word resolves `docx_serif`, `docx_sans`
  and `docx_mono` locally and may reflow the document after editing.
- The PDF block cover can fill the physical sheet. DOCX represents the same
  strategy as a reversed colour block over the printable area because Word
  does not expose a true full-bleed page background here. Cover artwork is the
  exception: it is anchored to the page, so it bleeds in both.
- PDF and Word comparison tables repeat their headers. The DOCX emitter adds
  `w:tblHeader` to header rows in the packaged OOXML.
- Page breaks and total page counts may consequently differ; content,
  hierarchy, labels, captions and diagram selection do not.

## Adding a theme

Copy a built-in as a starting point, edit, and select it:

```bash
mkdir -p ~/.config/azdocs/themes && cp data/themes/azure.toml ~/.config/azdocs/themes/acme.toml
```

```toml
[branding]
theme = "acme"
```

On macOS the config directory is
`~/Library/Application Support/azdocs/`; on Windows `%APPDATA%\azdocs\`.

Next: [Labels](labels.md)
