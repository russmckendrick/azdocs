---
version: 1
slug: "desktop-src-components-cytoscaperesourcegraph-tsx"
primary_target: "desktop/src/components/CytoscapeResourceGraph.tsx"
related_targets: ["desktop/src/components/TopologyView.tsx","desktop/src/App.tsx","desktop/src/styles.css"]
---

# Desktop estate explorer surface brief

- Mode: operate
- Primary users: Azure platform, cloud infrastructure, and security engineers.
- Primary job: move from a stored estate-wide picture to one exact resource,
  its derived dependencies, findings, tags, and raw properties without making
  another Azure request.
- Product truth: the Tauri webview receives typed DTOs from the Rust boundary;
  every exploration surface reads SQLite and never Azure directly.
- Visual thesis: a luminous cartographic field inside the Field Report. The
  Cytoscape.js estate map is the authored focal surface; its paper-like grid,
  restrained vignette, containment frames, and labelled rails make topology
  read as mapped evidence rather than a generic node-link canvas.
- Visual language: light is the canonical paper-and-ink plate and dark is its
  direct warm-charcoal remap. Native Microsoft service artwork stays
  transparent. Neutral tokens provide depth; relationship-family colour is
  reserved for connectors, trace marks, and label keys.
- Motion thesis: small moving edge dashes explain direction without obscuring
  the topology. Motion is controllable, pauses while hidden, and is removed
  for reduced-motion users.
- Interaction: resource-group estate overview, direct group drill-down,
  one-hop resource mode, node dragging, pan, zoom, group and resource
  selection, breadcrumbs, recentering, keyboard-reachable DOM node labels,
  global search, filters, and record navigation. The graph always owns the
  full workspace; resource evidence opens as a dedicated record and aggregate
  members expand in place rather than reserving an inspector column.
- Connector contract: every logical link owns invisible source and target
  ports on the real node or frame boundary. Shared sides are ordered by the
  opposite endpoint's position and fanned into deterministic orthogonal taxi
  channels. Logical ids remain authoritative for trace, camera, counts,
  navigation, and accessibility.
- Label contract: relationship text appears only for the active trace in an
  opaque DOM plate above every connector. It sits by the remote endpoint,
  receives a stable collision-aware slot, and resynchronises through pan,
  zoom, drag, resize, and camera animation. Cytoscape edge labels are banned.
- Entry contract: estate includes expanded lanes and collapsed subscription
  bars; groups include the connected core and external context but not the
  unconnected shelf; neighbourhoods include the subject and one-hop peers.
  Each scope starts wider and settles inward; Fit all remains explicit.
- Constraints: deterministic ordering; lowercase ARM IDs as join keys; no
  containment edges; honest drawn/folded/aggregated counts; no credentials,
  file access, database access, or Azure calls in the webview. Full contract:
  `docs/development/desktop-relationships.md`.
- Target viewport: desktop-first at 1440×900, usable down to the existing
  compact desktop breakpoint.
- Visual evidence: output/desktop-ui/topology-desktop.png and
  output/desktop-ui/estate-desktop.png.
