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
| Canvas | Snapped to a share of an A4 portrait page | Natural, 1400px working width |
| Consumers | PDF, DOCX, HTML report | `azdocs diagram` |

The summary is what makes a 59-resource group fit a page at a readable size.
The full export is what you open in draw.io when you need every name.

```sh
azdocs report  --format pdf        # summarised, page-fraction canvases
azdocs diagram --type resource-groups --format svg   # full detail
```

## Page fractions

Every summary diagram is emitted at exactly one of four canvas sizes, so a
report stacks predictable blocks rather than a run of rectangles that each
downscale by a different amount.

| Fraction | Canvas (px) | On paper |
|---|---|---|
| Quarter | 680 × 250 | 170 × 62.5 mm |
| Third | 680 × 333 | 170 × 83 mm |
| Half | 680 × 500 | 170 × 125 mm |
| Full | 680 × 1000 | 170 × 250 mm |

`PX_PER_MM` is exactly `4.0`, so one layout pixel is 0.25 mm and every integer
pixel is an exact millimetre quantity. The A4 portrait content box is
170 × 250 mm after the report's 20 mm margins and the room a caption needs.

The smallest fraction that holds the content without shrinking it below
`MIN_SCALE` wins. Content is never enlarged — a small diagram prints at true
size.

## Density rungs

The rung is chosen once per graph from its box count, so a resource type is
never drawn at two sizes in one picture.

| Rung | Boxes | Leaf | Icon | Label | Sublabel |
|---|---|---|---|---|---|
| `comfortable` | ≤ 14 | 152 × 112 | 48 | 11px | shown |
| `compact` | 15–32 | 116 × 88 | 40 | 10px | shown |
| `dense` | > 32 | 88 × 64 | 32 | 9px | shown |

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
- Edges leave from a **boundary anchor** on the side facing the target.
- Several edges meeting one box **fan out** along that side rather than piling
  onto the midpoint, turning a fan into a bus.
- Mid-segments **snap to an 8px lane grid** and deconflict, so parallel trunks
  stack.
- Corners are rounded 8px — what draw.io's own `rounded=1` draws, so the SVG
  and the `.drawio` agree.
- Labels sit on the **longest segment**, not the straight-line midpoint, which
  for a routed edge is frequently nowhere near the connector.

There is deliberately **no obstacle search**. Routing is closed-form and paid
once per edge per diagram, and a report renders hundreds of diagrams. A
connector may still cross an unrelated box; that is accepted.

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
| Peering disconnected | `#D13438` | Edge |
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
