# Mark prompts and design record

The raster boards in [`concepts/`](concepts/) record the earlier folded-A
exploration. The current rounded triangular mark replaces its facets, fold and
foreground graph with one silhouette and a white topology counter. The final simplified reference supplied by
the project owner was rebuilt as deterministic paths and shapes. Automatic
tracing was rejected because it would preserve raster blur and asymmetric edge
wobble instead of the intended clean geometry.

## Simplified production redraw

```text
Recreate the supplied rounded triangular blue A as a precise production mark.
Keep one symmetric outer silhouette with generously rounded crown and feet.
Inside it, place exactly three equal white circular nodes connected by one
white Y-shaped route. Use a restrained Azure-blue gradient only on the outer
shape. Remove the source raster's fuzzy edge shading, bevel and drop shadows.
The result must remain unmistakable at 16–32 px, work on light and dark launch
surfaces, and convert cleanly to SVG, PNG, ICO and ICNS.
```

## Four-direction exploration

```text
Use case: logo-brand
Asset type: brand exploration board for the azdocs Rust desktop application;
concepts must scale to app icon, favicon, document cover background, horizontal
logo lockup, and standalone mark.

Primary request: Create one polished comparison sheet containing four distinct
logo directions for “azdocs”, an application that audits Microsoft Azure
estates, stores snapshots, and exports field-report documents and architecture
diagrams. Each direction should fuse an abstract, faceted angular capital A
(Azure-adjacent, but not an exact copy of Microsoft's Azure trademark) with
blueprint, topology-diagram, or document-page elements.

Directions:
A. Folded Blueprint — angular A formed from two folded document planes;
   negative-space crossbar resembles a right-angle diagram connector.
B. Topology A — bold triangular A whose internal counter contains three
   connected blueprint nodes; extremely simple outer silhouette.
C. Survey Sheet — document/page outline with one folded corner; internal
   construction lines form an angular A and one orthogonal route.
D. Estate Atlas — modular A assembled from two clean resource tiles and a page
   strip, joined by one boundary-anchored connector.

Style: flat vector-logo aesthetic; geometric, precise, minimal, professional;
strong silhouette; solid fills; no gradients, 3D or shadows.
Palette: #1c2430, #0078d4, #0b5da8, #f3efe7, #faf8f4 and #14181d.
Text: exact lowercase “azdocs”.
Constraints: no official Microsoft/Azure logo copied exactly; no cloud, shield,
magnifying glass, checkmark, globe, watermark or decorative clutter. Each mark
must remain recognisable in monochrome and at tiny size.
```

## Flat presentation refinement

```text
Change only the visual rendering style of the four-concept board. Preserve the
same concepts, 2×2 layout, labels, app-icon tests, monochrome tests and exact
lowercase “azdocs” wordmarks. Remove every glow, bloom, blur, atmospheric haze,
gradient, bevel, 3D extrusion and drop shadow. Redraw the board as crisp flat
vector-style artwork using solid fills, warm paper #faf8f4, hairlines #d8d2c6,
ink #1c2430, Azure blue #0078d4, accent #0b5da8, evidence paper #f3efe7 and dark
icon backgrounds #14181d.
```

## Selected hybrid

Image 1 is the four-direction board and acts as the edit/reference image.

```text
Use case: precise-object-edit
Asset type: focused azdocs logo refinement board

Primary request: Combine Concept B's simple three-node topology motif inside
the A with Concept A's folded-document letterform. Make the A substantially
thicker and bolder than either original. Move the folded page detail to the
very top apex/crown of the A.

Subject: A single heavy, geometric, angular capital A. Its legs and shoulders
are broad solid shapes with a strong compact silhouette. At the top crown, a
small paper corner folds forward to reveal warm off-white beneath the Azure-blue
face. In the central negative-space counter, retain exactly three circular
topology nodes connected as a simple triangle, visually secondary to the A.

Show: large primary symbol; light and dark app icons; one-colour ink and paper
silhouettes; horizontal lockup with exact lowercase “azdocs”; and a subtle
oversized crop as a document-cover/background motif.

Style: crisp, flat, SVG-recreatable, geometric and editorial. Solid fills only.
Palette: #0078d4, #0b5da8, #5aa7e8, #1c2430, #faf8f4, #f3efe7 and #14181d.

Constraints: keep exactly three nodes; outer A must be visibly thick; fold must
be at the crown. No thin blueprint grid in the core icon. No gradients, glows,
blur, shadows, bevels, 3D, mockup photography, watermark or extra words. Do not
copy the official Microsoft Azure logo exactly.
```

## SVG regeneration constraints

Any future redraw should preserve these invariants:

- `0 0 512 512` standalone-mark coordinate system.
- Rounded triangular silhouette begins
  `M232 58 C242 38 270 38 280 58 L480 414` and remains symmetric.
- Exactly three 40-unit-radius nodes centred at `(256,198)`, `(156,382)` and
  `(356,382)`.
- One 26-unit Y connector follows
  `M256 198 V330 L156 382 M256 330 L356 382`.
- Nodes and connector use white or warm-paper negative space, never separate
  colours or foreground shadows.
- The only gradient is `#168fe5` through `#0b84dc` to `#0078d4` on the outer A.
- No fold, facet, crossbar, tile border, bevel, glow or drop shadow.
- The lockup uses the mark as its initial **A**, followed by an outlined `zdocs`
  from `data/fonts/IBMPlexSans-Bold.ttf`; never repeat a typed `a` beside
  the mark. Use deep blue on light surfaces and paper on dark surfaces.

The production source is
[`tools/build_vector_assets.py`](tools/build_vector_assets.py). Do not trace a
PNG to replace these paths; use the raster refinement only as an art-direction
reference.

## Continuous ribbon refinement

```text
Change only the A construction. Make both complete blue ribbon planes visibly
thicker. The deep-blue ribbon must run continuously from the lower-left foot to
the crown. The lighter cyan ribbon must begin at the crown and continue without
interruption down the full right leg to the lower-right foot. Keep the warm
paper fold at the upper-right crown and exactly three topology nodes.
```

## Foreground topology refinement

```text
Make both complete blue ribbon planes approximately 25% heavier while
preserving their continuous left-foot-to-crown and crown-to-right-foot paths.
Enlarge the three-node graph moderately, thicken all three connector bars by
about 30%, and move it forward and slightly lower so the two bottom nodes
overlap the inner edges of the ribbons. Draw nodes and connectors over the A.
Add only a tight, restrained shadow beneath the graph so it appears to hover.
Keep exactly three equal circular blue nodes and three straight connectors.
```
