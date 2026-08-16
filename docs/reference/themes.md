# Themes

Themes are data. A theme is a TOML file that supplies **values** — colours, a
type scale, and a choice from a closed set of layout strategies. Every emitter
implements each strategy once and branches only on those values, so a new
theme is a new TOML file, never new Rust.

```toml
# azdocs.toml
[branding]
theme = "fluent"
```

Built-in themes live in `data/themes/` and are embedded in the binary. Drop
files into `<config dir>/azdocs/themes/` to add your own or replace a built-in
of the same name — the same drop-in pattern as
[`queries.d/`](queries.md#user-queries). An unknown theme name is an error
that lists the names that do exist.

| Theme | Look |
|---|---|
| `fluent` | Azure-native. Colour band cover, filled table headers, zebra rows. |
| `editorial` | Consultancy audit report. Centred cover, chapter divider pages, hairline tables. |
| `dashboard` | Modern tech. Full-bleed colour block cover, tinted KPI cards, banded tables. |

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

# One block per severity: high, medium, low, info.
[palette.severity.high]
text = "#a4262c"
fill = "#f8cecc"

[typography]
sans      = "IBM Plex Sans"   # PDF + HTML; must be loadable (see Fonts below)
mono      = "IBM Plex Mono"
docx_sans = "Calibri"         # Word resolves by name on the reader's machine
docx_mono = "Consolas"
base_pt = 10.0
small_pt = 8.0
table_pt = 7.5
table_header_pt = 8.0
title_pt = 30.0
subtitle_pt = 13.0
h1_pt = 18.0
h2_pt = 13.0
h3_pt = 11.0
stat_value_pt = 19.0
stat_label_pt = 7.5
line_height = 1.4

[layout]
cover = "band"            # band | editorial | block
table = "solid-header"    # solid-header | hairline | banded
stat  = "card"            # card | outline | bare
heading_numbering = true
divider_pages = false     # a full page before each chapter
running_header = true
zebra_rows = true
rule_pt = 0.5
radius_pt = 4.0
table_inset_pt = 4.5
cover_band_pt = 96.0
```

### Layout strategies

These three keys are the contract between data and code. Each emitter
implements all of the variants; a theme picks one.

| Key | Variant | Effect |
|---|---|---|
| `cover` | `band` | Colour band across the top, content left-aligned below |
| | `editorial` | Centred type with a hairline rule, no fills |
| | `block` | Full-bleed colour block with a reversed-out title |
| `table` | `solid-header` | Filled header with reversed text, full grid |
| | `hairline` | Horizontal hairlines only, no header fill |
| | `banded` | Tinted header, horizontal rules only |
| `stat` | `card` | Tinted card with an accent top rule |
| | `outline` | Hairline-outlined box |
| | `bare` | No box: value over label |

## Fonts

The PDF sets its own type: `typst-assets` ships no proportional sans, so
[IBM Plex](https://github.com/IBM/plex) Sans and Mono are vendored in
`data/fonts/` under the SIL Open Font License 1.1.

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
nobody has installed lands back at a Word default. The shipped themes
therefore set `docx_sans = "Calibri"` and `docx_mono = "Consolas"`, which come
with Office on Windows and macOS. If your organisation deploys its own
typeface, override those two keys in a user theme.

## Adding a theme

Copy a built-in as a starting point, edit, and select it:

```bash
mkdir -p ~/.config/azdocs/themes && cp data/themes/fluent.toml ~/.config/azdocs/themes/acme.toml
```

```toml
[branding]
theme = "acme"
```

On macOS the config directory is
`~/Library/Application Support/azdocs/`; on Windows `%APPDATA%\azdocs\`.

Next: [Queries](queries.md)
