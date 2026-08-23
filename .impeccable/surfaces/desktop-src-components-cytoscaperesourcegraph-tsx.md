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
- Visual thesis: a spatial Azure observatory. The immersive Cytoscape.js
  estate map is the authored focal surface; inventory, findings, and history
  are quiet evidence workspaces around it.
- Visual language: deep nocturnal graph space, spectral Azure cyan, native
  Microsoft service artwork without white button backplates, transparent
  controls, fine relationship lines, and pale evidence surfaces.
- Motion thesis: small moving edge dashes explain direction without obscuring
  the topology. Motion is controllable, pauses while hidden, and is removed
  for reduced-motion users.
- Interaction: resource-group estate overview, explicit group drill-down,
  one-hop resource mode, node dragging, pan, zoom, group and resource
  selection, breadcrumbs, recentering, keyboard-reachable DOM node labels,
  global search, filters, and evidence inspection.
- Constraints: deterministic ordering; lowercase ARM IDs as join keys; no
  containment edges; orthogonal boundary-anchored connectors; no credentials,
  file access, database access, or Azure calls in the webview.
- Target viewport: desktop-first at 1440×900, usable down to the existing
  compact desktop breakpoint.
- Visual evidence: output/desktop-ui/topology-desktop.png and
  output/desktop-ui/estate-desktop.png.
