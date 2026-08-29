# Mark prompts and design record

The raster boards in [`concepts/`](concepts/) were generated with the built-in
image tool. The production SVGs were then reconstructed as deterministic paths
and shapes; they are not traced raster output.

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
- Flat top from x=188 to x=300; the fold begins at x=300 on that crown.
- Broad outer extents from x=40 to x=476 and y=64 to y=448.
- Exactly three equal nodes centred at `(256,270)`, `(215,350)` and `(297,350)`.
- One structural blue facet only; no name-dependent styling or gradients.
- Lockup wordmark outlined from the repository's
  `data/fonts/IBMPlexSans-SemiBold.ttf`.
