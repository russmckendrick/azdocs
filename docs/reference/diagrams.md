# Diagram standards

Diagrams are drawn from the snapshot only — never the network — and are
emitted as draw.io, Mermaid, SVG and PNG. This page is the contract every
emitter follows. It exists because the two consumers of a diagram want
opposite things, and the design turns on that split.

## Detail levels

A diagram embedded in a document has a page to respect. A diagram exported on
its own does not. One flag, `DiagramDetail`, decides both.

| | `Summary` | `Full` |
|---|---|---|
| Resources | Aggregated by type (`Storage Account ×13`) | One tile per resource |
| Canvas | Full text-column width, height from the content | Natural, 1400px working width |
| Consumers | PDF, DOCX, HTML report | `azdocs diagram` |

The summary is what makes a 59-resource group fit a page at a readable size.
The full export is what you open in draw.io when you need every name.

```sh
azdocs report  --format pdf        # summarised, page-width canvases
azdocs diagram --type resource-groups --format svg   # full detail
```

## Assessment figures use the shared pipeline

The default PDF/Word assessment selects focused relationship figures rather
than an exhaustive estate hierarchy. `graph/assessment.rs` creates
`EstateGraph`s, and `assets.rs` sends them through the same `page`, `layout`,
`route`, `text`, `icons`, `svg` and `png` components used elsewhere. Report composition
must never draw its own cards, positions, paths, colours or text wrapping.

Studies select up to two connection types, ranked by cross-group links and then
connection count. All links in selected types are retained, grouped by destination
and split at the node budget. A figure follows the sources feeding one destination
so crossing lines cannot imply an unrecorded junction.
Figures fold known NIC/disk/child attachments
with explicit counts, retain named external group frames, and split large
connection sets. Counts describe all resources represented by an aggregate,
not a claim that every member has every drawn connection. Captions and adjacent
evidence tables explain the scope. The optional technical reference retains
full resource-group diagrams and the complete registers.

## Page sizing

A summary diagram is **exactly the width of the text column** — 680px, the
170mm A4 content box — and as tall as its content needs at that width, capped
at the 250mm page height. `PX_PER_MM` is exactly `4.0`, so one layout pixel is
0.25mm and every integer pixel is an exact millimetre quantity.

The width sets the scale, so every diagram in a report downscales by the same
~0.95 and its labels print at the same size. The height was previously snapped
*up* to a quarter/third/half/full page, which let the height set the scale
instead — every diagram came out a shrunken block adrift in white space.
`PageFraction` survives to name the share a canvas claims (`data-page-fraction`
on the drawing); it no longer imposes it.

A label must reach the shared fitter intact; do not truncate it first.

The title band is a `Full`-export affair. In a report the section heading above
and the figure caption below already name the diagram, so `Summary` drops it
(`DiagramDetail::shows_title`). The legend band is likewise only reserved when
there are boundaries to explain.

## Density rungs

The rung is chosen once per graph from its box count, so a resource type is
never drawn at two sizes in one picture.

| Rung | Boxes | Leaf | Icon | Label | Sublabel |
|---|---|---|---|---|---|
| `comfortable` | ≤ 14 | 152 × 112 | 48 | 11px | shown |
| `compact` | 15–32 | 116 × 88 | 40 | 10px | shown |
| `dense` | > 32 | 88 × 64 | 32 | 9px | shown |

Labels are **fitted, never left to overflow**. A name is wrapped at separators
to the box width, and if it still does not fit it steps down a point at a time
to a floor of 8px before any cut is made — an Azure group name runs past thirty
characters, and `rg-n4-corp-dwh-…` on two tiles that differ only in what was
cut tells a reader nothing. A cut that does happen is always marked with an
ellipsis, which is also the signal the fitter tests. In a container band the
CIDR or count takes at most 40% of the width and shrinks to fit it, so the two
can never overprint.

Padding, gutter and title band decay by 2px per nesting level to a floor —
nested chrome otherwise compounds, and a resource group > VNet > subnet > leaf
shape pays its padding four times. Fonts and leaf sizes never decay.

## Layout

Two passes, in `diagram/layout.rs`:

1. **Measure** — post-order, every subtree at its natural size.
2. **Justify** — pre-order, every row stretched to exactly the width it was
   given, so containers span the sheet instead of huddling in the middle.

Column count is chosen by **lookahead**: halving the measure makes a child
re-flow into a narrower, taller shape, so a parent only goes side-by-side if
that reflow still comes out shorter. Height is the objective. Optimising for a
perfectly filled last row instead made layouts taller to avoid one empty cell.

## Connectors

Right-angled, always. Straight centre-to-centre lines were the original defect:
for a container the centre sits *inside* the box, so a VNet peering erupted
from the middle of a subnet and crossed the icons on its way out.

- **Containment edges are not drawn.** Nesting already says it.
- Edges leave from a **boundary anchor**. The four opposite and four matching side-pairs are
  scored and the one running through the **fewest boxes** wins, ties going to
  the pair geometry suggests. Two tiles in one row are "side by side", so the
  direct line was drawn straight through whatever sat between them; leaving
  through the top and running above the row is clear.
- A leaf is met **under its label** when an edge arrives from below. The label
  sits directly beneath the glyph, so anchoring on the glyph drew the connector
  through the name of the resource it pointed at.
- The **channel slides off** any box it would be drawn through, to the nearest
  free lane between the two anchors — a trunk runs down the gutter between two
  tiles rather than across one.
- Several edges meeting one box **fan out** along that side rather than piling
  onto the midpoint, turning a fan into a bus.
- Mid-segments **snap to an 8px lane grid** and deconflict, so parallel trunks
  stack. A lane adjustment is rejected if it would enter a box; narrow gaps
  keep bends between their boundaries even when shorter than the preferred escape.
- Corners are rounded 8px — what draw.io's own `rounded=1` draws, so the SVG
  and the `.drawio` agree.
- Labels prefer long horizontal segments, with positions scored against fitted node
  text, glyphs, container borders, heading bands and other labels. They stay in connector space rather than being
  shifted blindly into a nearby box.

There is deliberately **no path search**. Every step above is closed-form — a
fixed set of candidates scored against the boxes — because routing is paid once
per edge per diagram and a report renders hundreds of diagrams. A connector may
still cross an unrelated box; that is accepted.

## Zones and containers

A resource group splits into two zones, following the Azure reference
topologies:

- **Virtual network** — blue dashed border, resources nested in subnets.
- **Not in a virtual network** — orange dashed border, everything else,
  aggregated by type in `Summary`.

Headers are top-left, not centred: `[icon] Virtual network · name` with the
CIDR right-aligned in the same band. The kind reads as a quiet grey prefix so
the name is what the eye lands on. A centred header competes with the content
beneath it.

A per-resource **neighbourhood** diagram is drawn inside its resource group's
frame too, with the subject in the *middle* of the row: nearly every edge ends
on it, and from one end a connector has to cross the tiles in between.

Empty subnets collapse to a **strip** carrying just name and CIDR. The address
space being allocated is worth stating; a full-height empty box reads as a
rendering fault.

Resource tiles are **type-primary**: the Azure display name on top, the
resource name (or `×N`) below. Azure names are long and noisy; the type is
what a reader scans for.

Tiles beyond `MAX_TILES` (11) collapse into a single
`Other resources · N types · ×M` entry.

## Colour

| Element | Hex | Usage |
|---|---|---|
| VNet | `#0078D4` | Border and label |
| Subnet | `#8A8886` | Dashed border |
| Not in a VNet | `#D97706` / `#B45309` | Border / label |
| Resource group | `#8A8886` | Dashed border |
| Peering connected | `#107C10` | Edge |
| Peering disconnected / initiated | `#D13438` | Edge |
| Peering state unavailable | `#605E5C` | Neutral edge; no inferred failure |
| Text primary | `#323130` | Labels |
| Text secondary | `#605E5C` | Sublabels, CIDRs |

Each container kind has its own dash rhythm — subnet `4,3`, VNet and
unnetworked `7,4`, resource group `6,5` — so the legend can name them and a
reader tells them apart by border alone.

Every diagram carries a legend keyed to the kinds actually present, so a simple
diagram is not captioned with boundaries it does not use.

## Icons

Official Azure service icons, embedded as base64 data URIs so an SVG needs no
external assets. Icon selection is data: `data/icons.toml`, with the display
names in `data/display_names.toml`. Adding a resource type is a TOML edit,
never Rust. See [Queries](queries.md) for the same pattern applied to the query
pack.

## Determinism

Golden tests and `$skipToken` pagination both depend on it:

- Routing uses `BTreeMap` and `total_cmp` with index tie-breaks, never
  `HashMap` iteration order.
- Fan-out graphs are sorted by slug.
- draw.io single-sheet cell ids stay bare `n{i}`/`e{i}` with sheet id
  `azdocs-0`; workbook sheets prefix `s{i}-`. Geometry moves freely with any
  layout change — the **id scheme** is what is frozen, not the bytes.

## Caps

| Cap | Value | Behaviour |
|---|---|---|
| `MAX_TILES` | 11 | Remainder collapses to one tile |
| `MAX_GROUP_DIAGRAMS` | 60 | Remainder reported without a diagram, logged |
| `MAX_RESOURCE_DIAGRAMS` | 250 | Remainder reported without a diagram, logged |

No cap is silent — each logs a warning naming the total and the cap.
