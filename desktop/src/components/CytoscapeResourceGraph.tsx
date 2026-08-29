import { useEffect, useMemo, useRef, useState } from "react";
import cytoscape, { type Core, type ElementDefinition, type StylesheetJson } from "cytoscape";
import { RESOURCE_GROUP_ICON, SUBSCRIPTION_ICON, VNET_ICON } from "../azure-icons";
import type { EstateSnapshot, TopologyGraph, TopologyNode } from "../types";

export type GraphMode = "neighbourhood" | "estate";

interface CytoscapeResourceGraphProps {
  graph: TopologyGraph;
  estate: EstateSnapshot;
  theme: "light" | "dark";
  focusedNodeId: string;
  motionEnabled: boolean;
  focusNonce: number;
  onSelectResource: (id: string) => void;
  onSelectResourceGroup: (id: string) => void;
  onOpenResourceGroup: (id: string) => void;
  onExpandLane: (subscriptionId: string) => void;
  onSelectAggregate: (nodeId: string) => void;
}

const RESOURCE_W = 172;
const RESOURCE_H = 76;
const GROUP_W = 244;
const GROUP_H = 118;
const AGGREGATE_W = 188;
const AGGREGATE_H = 60;
const LANE_BAR_W = 860;
const LANE_BAR_H = 56;

const KIND_CLASSES = new Set(["network", "structure", "data", "identity", "monitoring"]);

/// Every colour the canvas paints comes from the design-language tokens in
/// styles.css, resolved at graph build time so both themes use one palette.
function cssToken(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

export function kindClassColor(kindClass: string) {
  return cssToken(KIND_CLASSES.has(kindClass) ? `--kind-${kindClass}` : "--cat-other", "#5a6470");
}

interface GraphPalette {
  nodeFill: string;
  nodeBorder: string;
  frameBorder: string;
  focusFill: string;
  focusBorder: string;
  labelText: string;
  labelBg: string;
  labelBorder: string;
}

function readGraphPalette(): GraphPalette {
  return {
    nodeFill: cssToken("--graph-node-fill", "#ffffff"),
    nodeBorder: cssToken("--graph-node-border", "#d8d2c6"),
    frameBorder: cssToken("--graph-frame-border", "#9aa2ac"),
    focusFill: cssToken("--graph-focus-fill", "#f3efe7"),
    focusBorder: cssToken("--graph-focus-border", "#0b5da8"),
    labelText: cssToken("--graph-label-text", "#5a6470"),
    labelBg: cssToken("--graph-label-bg", "#faf8f4"),
    labelBorder: cssToken("--graph-label-border", "#d8d2c6"),
  };
}

interface Placement {
  x: number;
  y: number;
}

/// Deterministic positions for every positionable node. Containers (lane,
/// vnet, subnet compounds) are not positioned — cytoscape fits them around
/// their children.
function layoutPositions(graph: TopologyGraph): Map<string, Placement> {
  const positions = new Map<string, Placement>();
  if (graph.level === "estate") return layoutEstate(graph, positions);
  if (graph.level === "group") return layoutGroup(graph, positions);
  return layoutNeighbourhood(graph, positions);
}

function grid(count: number, columns: number, index: number, width: number, height: number, gapX: number, gapY: number) {
  const column = index % columns;
  const row = Math.floor(index / columns);
  return { x: column * (width + gapX), y: row * (height + gapY), rows: Math.ceil(count / columns) };
}

function layoutEstate(graph: TopologyGraph, positions: Map<string, Placement>) {
  const gapX = 56;
  const gapY = 58;
  // Wide lanes: enough columns that even a busy lane stays shallower than it
  // is wide, so zoom-to-fit keeps cards legible.
  const largestLane = Math.max(
    1,
    ...graph.lanes
      .filter((lane) => lane.expanded)
      .map((lane) => graph.nodes.filter((node) => node.kind === "resource-group" && node.lane === lane.subscriptionId).length),
  );
  const columns = Math.min(6, Math.max(3, Math.ceil(Math.sqrt(largestLane * 2.2))));
  const laneWidth = columns * (GROUP_W + gapX);
  let cursorY = 0;
  for (const lane of graph.lanes.filter((candidate) => candidate.expanded)) {
    const cards = graph.nodes.filter((node) => node.kind === "resource-group" && node.lane === lane.subscriptionId);
    if (cards.length === 0) continue;
    let rows = 1;
    cards.forEach((card, index) => {
      const cell = grid(cards.length, columns, index, GROUP_W, GROUP_H, gapX, gapY);
      rows = cell.rows;
      positions.set(card.id, { x: cell.x, y: cursorY + cell.y });
    });
    cursorY += rows * (GROUP_H + gapY) + 96;
  }
  for (const node of graph.nodes.filter((candidate) => candidate.kind === "subscription")) {
    positions.set(node.id, { x: Math.max(laneWidth, LANE_BAR_W) / 2 - LANE_BAR_W / 2, y: cursorY });
    cursorY += LANE_BAR_H + 34;
  }
  return positions;
}

function layoutGroup(graph: TopologyGraph, positions: Map<string, Placement>) {
  const childrenOf = new Map<string, TopologyNode[]>();
  for (const node of graph.nodes) {
    if (!node.parentId) continue;
    childrenOf.set(node.parentId, [...(childrenOf.get(node.parentId) ?? []), node]);
  }

  // VNet blocks down the left edge: subnets stacked, members in a grid.
  let vnetY = 0;
  let vnetColumnWidth = 0;
  for (const vnet of graph.nodes.filter((node) => node.kind === "vnet")) {
    let subnetY = vnetY + 64;
    for (const subnet of childrenOf.get(vnet.id) ?? []) {
      const members = childrenOf.get(subnet.id) ?? [];
      if (members.length === 0) {
        positions.set(subnet.id, { x: 190, y: subnetY + 20 });
        subnetY += 78;
        continue;
      }
      const columns = Math.min(3, Math.max(1, Math.ceil(Math.sqrt(members.length))));
      let rows = 1;
      members.forEach((member, index) => {
        const cell = grid(members.length, columns, index, RESOURCE_W, RESOURCE_H, 36, 30);
        rows = cell.rows;
        positions.set(member.id, { x: 60 + cell.x, y: subnetY + 44 + cell.y });
      });
      const width = 60 + columns * (RESOURCE_W + 36) + 40;
      vnetColumnWidth = Math.max(vnetColumnWidth, width);
      subnetY += rows * (RESOURCE_H + 30) + 96;
    }
    vnetY = subnetY + 110;
  }
  if (vnetColumnWidth === 0 && graph.nodes.some((node) => node.kind === "vnet")) vnetColumnWidth = 460;

  // Ghost stubs for other groups' resources sit in their own column on the
  // left, so cross-group traffic reads as arriving from outside.
  const externals = graph.nodes.filter((node) => node.zone === "external");
  const externalShift = externals.length > 0 ? RESOURCE_W + 220 : 0;
  externals.forEach((node, index) => {
    positions.set(node.id, { x: -externalShift, y: index * (RESOURCE_H + 40) });
  });

  // Free connected nodes flow in a grid beside the VNet column.
  const freeX = vnetColumnWidth > 0 ? vnetColumnWidth + 140 : 0;
  const free = graph.nodes.filter(
    (node) => node.kind === "resource" && !node.parentId && node.zone === "core",
  );
  const freeColumns = Math.min(4, Math.max(1, Math.ceil(Math.sqrt(free.length * 1.6))));
  free.forEach((node, index) => {
    const cell = grid(free.length, freeColumns, index, RESOURCE_W, RESOURCE_H, 64, 64);
    positions.set(node.id, { x: freeX + cell.x, y: cell.y });
  });

  // The unconnected shelf: a compact grid to the right of everything.
  const shelf = graph.nodes.filter((node) => node.zone === "unconnected");
  const shelfColumns = shelf.length > 6 ? 2 : 1;
  const shelfX = Math.max(freeX + freeColumns * (RESOURCE_W + 64) + 120, vnetColumnWidth + 140);
  shelf.forEach((node, index) => {
    const column = index % shelfColumns;
    const row = Math.floor(index / shelfColumns);
    positions.set(node.id, {
      x: shelfX + column * (AGGREGATE_W + 40),
      y: row * (AGGREGATE_H + 26),
    });
  });
  return positions;
}

function layoutNeighbourhood(graph: TopologyGraph, positions: Map<string, Placement>) {
  const subject = graph.nodes.find((node) => node.hop === 0);
  const ring = (hop: number) => graph.nodes.filter((node) => (node.hop ?? 0) === hop && node.hop !== 0);
  if (subject) positions.set(subject.id, { x: 0, y: 0 });
  for (const hop of [1, 2]) {
    const members = ring(hop);
    const perColumn = Math.max(1, Math.ceil(members.length / 2));
    members.forEach((node, index) => {
      const side = index % 2 === 0 ? 1 : -1;
      const rank = Math.floor(index / 2);
      const x = side * hop * 340;
      const y = (rank - (perColumn - 1) / 2) * (RESOURCE_H + 44);
      positions.set(node.id, { x, y });
    });
  }
  return positions;
}

function graphStyles(palette: GraphPalette): StylesheetJson {
  return [
    {
      selector: "node.resource-node",
      style: {
        width: RESOURCE_W,
        height: RESOURCE_H,
        shape: "round-rectangle",
        "corner-radius": "4px",
        "background-color": palette.nodeFill,
        "background-opacity": 1,
        "border-width": 1,
        "border-color": palette.nodeBorder,
        "border-opacity": 1,
        "overlay-opacity": 0,
        "underlay-opacity": 0,
        label: "",
        "z-index": 8,
      },
    },
    {
      selector: "node.resource-group-node",
      style: {
        width: GROUP_W,
        height: GROUP_H,
        shape: "round-rectangle",
        "corner-radius": "6px",
        "background-color": palette.nodeFill,
        "background-opacity": 1,
        "border-width": 1,
        "border-color": palette.nodeBorder,
        "border-opacity": 1,
        "overlay-opacity": 0,
        label: "",
        "z-index": 8,
      },
    },
    {
      selector: "node.aggregate-node",
      style: {
        width: AGGREGATE_W,
        height: AGGREGATE_H,
        shape: "round-rectangle",
        "corner-radius": "4px",
        "background-color": palette.nodeFill,
        "background-opacity": 0.7,
        "border-width": 1,
        "border-color": palette.frameBorder,
        "border-style": "dashed",
        "border-opacity": 1,
        "overlay-opacity": 0,
        label: "",
        "z-index": 8,
      },
    },
    {
      selector: "node.external-node",
      style: {
        width: RESOURCE_W,
        height: RESOURCE_H,
        shape: "round-rectangle",
        "corner-radius": "4px",
        "background-color": palette.nodeFill,
        "background-opacity": 0.5,
        "border-width": 1,
        "border-color": palette.frameBorder,
        "border-style": "dashed",
        "border-opacity": 0.9,
        "overlay-opacity": 0,
        label: "",
        "z-index": 6,
      },
    },
    {
      selector: "node.lane-bar-node",
      style: {
        width: LANE_BAR_W,
        height: LANE_BAR_H,
        shape: "round-rectangle",
        "corner-radius": "4px",
        "background-color": palette.nodeFill,
        "background-opacity": 0.7,
        "border-width": 1,
        "border-color": palette.frameBorder,
        "border-style": "dashed",
        "border-opacity": 0.9,
        "overlay-opacity": 0,
        label: "",
        "z-index": 8,
      },
    },
    {
      selector: "node.lane-frame, node.vnet-frame, node.subnet-frame",
      style: {
        shape: "round-rectangle",
        "corner-radius": "6px",
        "background-color": palette.nodeFill,
        "background-opacity": 0,
        "border-width": 1.2,
        "border-style": "dashed",
        "border-color": palette.frameBorder,
        "border-opacity": 0.85,
        "overlay-opacity": 0,
        label: "",
        "z-index": 1,
        // Room for the DOM header label above the children.
        "padding-top": "52px",
        "padding-left": "22px",
        "padding-right": "22px",
        "padding-bottom": "22px",
      },
    },
    {
      selector: "node.subnet-frame",
      style: {
        "corner-radius": "4px",
        "border-opacity": 0.6,
        "padding-top": "42px",
      },
    },
    {
      selector: "node.subnet-empty",
      style: {
        width: 260,
        height: 44,
        shape: "round-rectangle",
        "corner-radius": "4px",
        "background-color": palette.nodeFill,
        "background-opacity": 0.4,
        "border-width": 1,
        "border-style": "dashed",
        "border-color": palette.frameBorder,
        "border-opacity": 0.6,
        label: "",
        "z-index": 4,
      },
    },
    {
      selector: "node.hop-dim",
      style: { "background-opacity": 0.55, "border-opacity": 0.55 },
    },
    {
      selector: "node.focused",
      style: {
        "background-color": palette.focusFill,
        "background-opacity": 1,
        "border-width": 1.5,
        "border-color": palette.focusBorder,
        "border-opacity": 1,
        "z-index": 14,
      },
    },
    {
      selector: "node.hovered",
      style: { "border-width": 1.5, "border-color": palette.focusBorder },
    },
    {
      selector: "node:grabbed, node.keyboard-focus",
      style: { "border-width": 2, "border-color": palette.focusBorder },
    },
    {
      selector: "edge.relationship-edge",
      style: {
        width: "mapData(weight, 1, 20, 1.35, 3.4)",
        "curve-style": "round-taxi",
        "taxi-direction": "horizontal",
        "taxi-turn": "50%",
        "taxi-turn-min-distance": "28px",
        "taxi-radius": 9,
        "edge-distances": "intersection",
        "line-color": "data(color)",
        "line-opacity": 0.75,
        "line-cap": "round",
        "target-arrow-shape": "triangle",
        "target-arrow-color": "data(color)",
        "arrow-scale": 0.72,
        "source-distance-from-node": "5px",
        "target-distance-from-node": "7px",
        "overlay-opacity": 0,
        events: "no",
        "z-index": 2,
      },
    },
    {
      selector: "edge.relationship-edge.related",
      style: {
        width: 2,
        "line-opacity": 1,
        "line-style": "dashed",
        "line-dash-pattern": [7, 5],
      },
    },
    {
      selector: "edge.relationship-edge.labelled",
      style: {
        label: "data(label)",
        color: palette.labelText,
        "font-family": "Plex Mono, monospace",
        "font-size": 8,
        "font-weight": 600,
        "text-transform": "uppercase",
        "text-background-color": palette.labelBg,
        "text-background-opacity": 0.95,
        "text-background-padding": "4px",
        "text-background-shape": "roundrectangle",
        "text-border-color": palette.labelBorder,
        "text-border-opacity": 0.8,
        "text-border-width": 1,
      },
    },
  ];
}

/// Fit must never zoom IN past ~1:1 — a three-node graph blown up to fill the
/// stage reads as broken, not sparse.
const MAX_FIT_ZOOM = 0.95;

function clampFitZoom(cy: Core, around?: cytoscape.CollectionReturnValue) {
  if (cy.zoom() > MAX_FIT_ZOOM) {
    cy.zoom(MAX_FIT_ZOOM);
    cy.center(around && around.nonempty() ? around : cy.elements());
  }
}

function nodeClasses(node: TopologyNode, focusedNodeId: string) {
  const classes: string[] = [];
  if (node.kind === "resource") classes.push("resource-node");
  if (node.kind === "resource-group") classes.push("resource-group-node");
  if (node.kind === "aggregate") classes.push("aggregate-node");
  if (node.kind === "external") classes.push("external-node");
  if (node.kind === "subscription") classes.push("lane-bar-node");
  if (node.kind === "vnet") classes.push("vnet-frame");
  if (node.kind === "subnet") classes.push("subnet-frame");
  if ((node.hop ?? 0) > 1) classes.push("hop-dim");
  if (node.id === focusedNodeId) classes.push("focused");
  return classes.join(" ");
}

function graphElements(graph: TopologyGraph, focusedNodeId: string): ElementDefinition[] {
  const childCount = new Map<string, number>();
  for (const node of graph.nodes) {
    if (node.parentId) childCount.set(node.parentId, (childCount.get(node.parentId) ?? 0) + 1);
  }
  const laneHasCards = new Set(
    graph.nodes.filter((node) => node.kind === "resource-group" && node.lane).map((node) => node.lane as string),
  );
  const elements: ElementDefinition[] = [];
  for (const lane of graph.lanes.filter((candidate) => candidate.expanded)) {
    if (!laneHasCards.has(lane.subscriptionId)) continue;
    elements.push({
      group: "nodes",
      data: { id: `lane:${lane.subscriptionId}`, kind: "lane" },
      classes: "lane-frame",
      selectable: false,
      grabbable: false,
    });
  }
  for (const node of graph.nodes) {
    const isEmptySubnet = node.kind === "subnet" && (childCount.get(node.id) ?? 0) === 0;
    const parent =
      node.parentId ??
      (graph.level === "estate" && node.kind === "resource-group" && node.lane && laneHasCards.has(node.lane)
        ? `lane:${node.lane}`
        : undefined);
    elements.push({
      group: "nodes",
      data: {
        id: node.id,
        kind: node.kind,
        parent,
        resourceId: node.resourceId,
        groupId: node.groupId,
        lane: node.lane,
      },
      classes: isEmptySubnet ? "subnet-empty" : nodeClasses(node, focusedNodeId),
      selectable: node.kind !== "vnet" && node.kind !== "subnet",
      grabbable: node.kind === "resource" || node.kind === "aggregate" || node.kind === "external",
    });
  }
  graph.links.forEach((link, index) => {
    const related = link.sourceId === focusedNodeId || link.targetId === focusedNodeId;
    elements.push({
      group: "edges",
      data: {
        id: `relationship-${index}`,
        source: link.sourceId,
        target: link.targetId,
        label: link.count > 1 && graph.level === "estate" ? `${link.count} links` : link.label,
        weight: link.count,
        color: kindClassColor(link.kindClass),
      },
      classes: [
        "relationship-edge",
        related ? "related" : "",
        graph.level !== "group" || related ? "labelled" : "",
      ]
        .filter(Boolean)
        .join(" "),
    });
  });
  return elements;
}

export function CytoscapeResourceGraph({
  graph,
  estate,
  theme,
  focusedNodeId,
  motionEnabled,
  focusNonce,
  onSelectResource,
  onSelectResourceGroup,
  onOpenResourceGroup,
  onExpandLane,
  onSelectAggregate,
}: CytoscapeResourceGraphProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const surfaceRef = useRef<HTMLDivElement>(null);
  const labelRefs = useRef(new Map<string, HTMLElement>());
  const cyRef = useRef<Core | undefined>(undefined);
  const callbacksRef = useRef({ onSelectResource, onSelectResourceGroup, onOpenResourceGroup, onExpandLane, onSelectAggregate });
  const motionRef = useRef(motionEnabled);
  const [rendererError, setRendererError] = useState<string>();
  const typeMap = useMemo(
    () => new Map(estate.resourceTypes.map((type) => [type.azureType, type])),
    [estate.resourceTypes],
  );
  const graphStructureKey = useMemo(
    () =>
      [
        graph.level,
        graph.nodes.map((node) => `${node.id}~${node.parentId ?? ""}~${node.kind}`).join("|"),
        graph.links.map((link) => `${link.sourceId}~${link.targetId}~${link.kindClass}~${link.count}`).join("|"),
        graph.lanes.map((lane) => `${lane.subscriptionId}~${lane.expanded}`).join("|"),
      ].join("\n"),
    [graph],
  );

  useEffect(() => {
    callbacksRef.current = { onSelectResource, onSelectResourceGroup, onOpenResourceGroup, onExpandLane, onSelectAggregate };
  }, [onExpandLane, onOpenResourceGroup, onSelectAggregate, onSelectResource, onSelectResourceGroup]);

  useEffect(() => {
    motionRef.current = motionEnabled;
  }, [motionEnabled]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    cy.batch(() => {
      cy.nodes().removeClass("focused");
      cy.getElementById(focusedNodeId).addClass("focused");
      cy.edges(".relationship-edge").forEach((edge) => {
        const related = edge.source().id() === focusedNodeId || edge.target().id() === focusedNodeId;
        edge.toggleClass("related", related);
      });
    });
  }, [focusedNodeId]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy || focusNonce === 0) return;
    const selected = cy.getElementById(focusedNodeId);
    if (selected.empty()) return;
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    cy.stop();
    if (reduceMotion) {
      cy.fit(selected, 150);
      clampFitZoom(cy, selected);
      return;
    }
    cy.animate(
      { fit: { eles: selected, padding: 150 } },
      { duration: 420, easing: "ease-out-cubic", complete: () => clampFitZoom(cy, selected) },
    );
  }, [focusNonce, focusedNodeId]);

  useEffect(() => {
    const surface = surfaceRef.current;
    const host = hostRef.current;
    if (!surface || !host || graph.nodes.length === 0) return;
    const activeHost = host;

    let cy: Core;
    try {
      cy = cytoscape({
        container: surface,
        elements: graphElements(graph, focusedNodeId),
        style: graphStyles(readGraphPalette()),
        layout: { name: "preset" },
        minZoom: 0.1,
        maxZoom: 2.2,
        boxSelectionEnabled: true,
        selectionType: "single",
        pixelRatio: "auto",
      });
      cyRef.current = cy;
      setRendererError(undefined);
    } catch (error) {
      setRendererError(error instanceof Error ? error.message : "The relationship graph could not be initialised.");
      return;
    }

    let labelFrame = 0;
    let resizeFrame = 0;
    let motionFrame = 0;
    let lastMotionPaint = 0;
    let dashOffset = 0;
    let visible = document.visibilityState === "visible";
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const frameIds = new Set(
      graph.nodes.filter((node) => node.kind === "vnet" || node.kind === "subnet").map((node) => node.id),
    );
    for (const lane of graph.lanes.filter((candidate) => candidate.expanded)) frameIds.add(`lane:${lane.subscriptionId}`);

    function syncLabels() {
      labelFrame = 0;
      const zoom = cy.zoom();
      // Frame headers (lane, vnet, subnet) stay readable when zoomed out —
      // they name whole regions, so they get a scale floor.
      const frameLabelScale = Math.max(zoom, 0.72);
      for (const [id, label] of labelRefs.current) {
        const node = cy.getElementById(id);
        if (node.empty()) continue;
        if (frameIds.has(id) && !node.hasClass("subnet-empty")) {
          const box = node.renderedBoundingBox({ includeLabels: false, includeOverlays: false });
          label.style.transform = `translate3d(${box.x1 + 14 * zoom}px, ${box.y1 + 6 * zoom}px, 0) scale(${frameLabelScale})`;
        } else {
          const point = node.renderedPosition();
          // Scale BEFORE the -50% centring so the offset is computed in the
          // scaled visual size — the other order drifts labels off their
          // boxes at any zoom other than 1:1.
          label.style.transform = `translate3d(${point.x}px, ${point.y}px, 0) scale(${zoom}) translate(-50%, -50%)`;
        }
      }
    }

    function queueLabelSync() {
      if (labelFrame !== 0) return;
      labelFrame = window.requestAnimationFrame(syncLabels);
    }

    function animateFlow(time: number) {
      motionFrame = window.requestAnimationFrame(animateFlow);
      if (!visible || reduceMotion.matches || !motionRef.current || time - lastMotionPaint < 38) return;
      lastMotionPaint = time;
      dashOffset = (dashOffset - 0.7) % 12;
      cy.edges(".relationship-edge.related").style("line-dash-offset", dashOffset);
    }

    function handleVisibility() {
      visible = document.visibilityState === "visible";
    }

    function handleTap(event: cytoscape.EventObject) {
      const kind = event.target.data("kind");
      if (kind === "resource-group") {
        callbacksRef.current.onSelectResourceGroup(event.target.data("groupId"));
        return;
      }
      if (kind === "subscription") {
        callbacksRef.current.onExpandLane(event.target.data("lane"));
        return;
      }
      if (kind === "aggregate") {
        callbacksRef.current.onSelectAggregate(event.target.id());
        return;
      }
      if (kind === "external") {
        const resourceId = event.target.data("resourceId");
        if (resourceId) callbacksRef.current.onSelectResource(resourceId);
        else callbacksRef.current.onSelectAggregate(event.target.id());
        return;
      }
      if (kind === "resource" || kind === "vnet") {
        const resourceId = event.target.data("resourceId");
        if (resourceId) callbacksRef.current.onSelectResource(resourceId);
      }
    }

    function handleDoubleTap(event: cytoscape.EventObject) {
      const groupId = event.target.data("groupId");
      if (groupId) callbacksRef.current.onOpenResourceGroup(groupId);
    }

    function handleNodeOver(event: cytoscape.EventObject) {
      if (frameIds.has(event.target.id())) return;
      event.target.addClass("hovered");
      activeHost.style.cursor = "pointer";
    }

    function handleNodeOut(event: cytoscape.EventObject) {
      event.target.removeClass("hovered");
      activeHost.style.cursor = "grab";
    }

    cy.on("tap", "node", handleTap);
    cy.on("dbltap", "node.resource-group-node", handleDoubleTap);
    cy.on("mouseover", "node", handleNodeOver);
    cy.on("mouseout", "node", handleNodeOut);
    cy.on("grab", "node", () => {
      activeHost.style.cursor = "grabbing";
    });
    cy.on("free", "node", () => {
      activeHost.style.cursor = "pointer";
    });
    cy.on("pan zoom position resize", queueLabelSync);
    document.addEventListener("visibilitychange", handleVisibility);

    // Until the user pans or zooms themselves, keep the graph fitted and
    // centred through container resizes — the stage often settles its final
    // size a frame after the graph mounts.
    let userAdjusted = false;
    const markUserAdjusted = () => {
      userAdjusted = true;
    };
    surface.addEventListener("pointerdown", markUserAdjusted);
    surface.addEventListener("wheel", markUserAdjusted, { passive: true });

    try {
      const positions = layoutPositions(graph);
      cy.batch(() => {
        for (const [id, placement] of positions) {
          const node = cy.getElementById(id);
          if (!node.empty() && !node.isParent()) node.position({ x: placement.x, y: placement.y });
        }
      });
      cy.fit(cy.elements(), 90);
      clampFitZoom(cy);
      queueLabelSync();
      if (focusNonce > 0 && graph.level !== "estate") {
        const selected = cy.getElementById(focusedNodeId);
        if (!selected.empty()) {
          cy.fit(selected, 150);
          clampFitZoom(cy, selected);
        }
      }
    } catch (error) {
      setRendererError(error instanceof Error ? error.message : "The relationship layout could not be calculated.");
      cyRef.current = undefined;
      cy.destroy();
      return;
    }

    const resizeObserver = new ResizeObserver(() => {
      cy.resize();
      window.cancelAnimationFrame(resizeFrame);
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = 0;
        if (!userAdjusted) {
          cy.fit(cy.elements(), 90);
          clampFitZoom(cy);
        }
        queueLabelSync();
      });
    });
    resizeObserver.observe(activeHost);
    motionFrame = window.requestAnimationFrame(animateFlow);
    queueLabelSync();

    return () => {
      window.cancelAnimationFrame(labelFrame);
      window.cancelAnimationFrame(resizeFrame);
      window.cancelAnimationFrame(motionFrame);
      resizeObserver.disconnect();
      surface.removeEventListener("pointerdown", markUserAdjusted);
      surface.removeEventListener("wheel", markUserAdjusted);
      document.removeEventListener("visibilitychange", handleVisibility);
      cyRef.current = undefined;
      cy.destroy();
    };
    // The graph rebuilds when its structure OR the resolved theme changes —
    // the palette and per-edge colours are read from CSS tokens at build time.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graphStructureKey, theme]);

  const registerLabel = (id: string) => (element: HTMLElement | null) => {
    if (element) labelRefs.current.set(id, element);
    else labelRefs.current.delete(id);
  };

  function typeIcon(azureType?: string) {
    const type = azureType ? typeMap.get(azureType) : undefined;
    return type?.icon;
  }

  function typeName(azureType?: string) {
    const type = azureType ? typeMap.get(azureType) : undefined;
    return type?.displayName ?? azureType ?? "Resource";
  }

  return (
    <div className="cytoscape-graph" ref={hostRef}>
      <div className="graph-aurora" aria-hidden="true" />
      <div className="cytoscape-surface" ref={surfaceRef} aria-hidden="true" />
      <div className="graph-label-layer">
        {graph.lanes
          .filter((lane) => lane.expanded && graph.nodes.some((node) => node.kind === "resource-group" && node.lane === lane.subscriptionId))
          .map((lane) => (
            <span key={`lane:${lane.subscriptionId}`} ref={registerLabel(`lane:${lane.subscriptionId}`)} className="graph-container-label">
              <img src={SUBSCRIPTION_ICON} alt="" />
              <strong>{lane.name}</strong>
              <small>
                {lane.groupCount} groups · {lane.resourceCount} resources
              </small>
            </span>
          ))}
        {graph.nodes.map((node) => {
          if (node.kind === "vnet") {
            return (
              <span key={node.id} ref={registerLabel(node.id)} className="graph-container-label graph-vnet-label">
                <img src={VNET_ICON} alt="" />
                <strong>{node.name}</strong>
                <small>{node.subtitle}</small>
              </span>
            );
          }
          if (node.kind === "subnet") {
            return (
              <span key={node.id} ref={registerLabel(node.id)} className="graph-container-label graph-subnet-label">
                <strong>{node.name}</strong>
                <small>{node.subtitle}</small>
              </span>
            );
          }
          if (node.kind === "subscription") {
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className="graph-lane-bar-label"
                onClick={() => onExpandLane(node.lane ?? "")}
                aria-label={`${node.name}, collapsed subscription. Press Enter to expand.`}
              >
                <img src={SUBSCRIPTION_ICON} alt="" />
                <span className="graph-node-copy">
                  <strong>{node.name}</strong>
                  <small>{node.subtitle}</small>
                </span>
                {node.findingCount > 0 ? <em>{node.findingCount}</em> : null}
                <i className="graph-lane-expand">Expand ›</i>
              </button>
            );
          }
          if (node.kind === "aggregate") {
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className="graph-aggregate-label"
                onClick={() => onSelectAggregate(node.id)}
                aria-label={`${node.count} ${typeName(node.azureType)}, aggregated. Press Enter to list them.`}
              >
                {typeIcon(node.azureType) ? <img src={typeIcon(node.azureType)} alt="" /> : null}
                <span className="graph-node-copy">
                  <strong>{typeName(node.azureType)}</strong>
                  <small>{node.zone === "unconnected" ? "not linked yet" : `${node.hop ?? 1} hop${(node.hop ?? 1) === 1 ? "" : "s"} away`}</small>
                </span>
                <b>{node.subtitle}</b>
                {node.findingCount > 0 ? <em>{node.findingCount}</em> : null}
              </button>
            );
          }
          if (node.kind === "external") {
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className="graph-node-label ghost"
                onClick={() => (node.resourceId ? onSelectResource(node.resourceId) : onSelectAggregate(node.id))}
                aria-label={`${node.name}, in another resource group`}
              >
                <span className="graph-node-icon">
                  {typeIcon(node.azureType) ? <img src={typeIcon(node.azureType)} alt="" /> : null}
                </span>
                <span className="graph-node-copy">
                  <strong>{node.count > 1 ? typeName(node.azureType) : node.name}</strong>
                  <small>{node.subtitle || "in another group"}</small>
                </span>
              </button>
            );
          }
          if (node.kind === "resource-group") {
            const selected = node.id === focusedNodeId;
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className={selected ? "graph-resource-group-label selected" : "graph-resource-group-label"}
                onClick={() => onSelectResourceGroup(node.groupId ?? "")}
                onDoubleClick={() => onOpenResourceGroup(node.groupId ?? "")}
                onKeyDown={(event) => {
                  if (event.key !== "Enter") return;
                  event.preventDefault();
                  onOpenResourceGroup(node.groupId ?? "");
                }}
                onFocus={() => cyRef.current?.getElementById(node.id).addClass("keyboard-focus")}
                onBlur={() => cyRef.current?.getElementById(node.id).removeClass("keyboard-focus")}
                aria-label={`${node.name}, ${node.count} resources. Press Enter to open.`}
              >
                <span className="graph-group-heading">
                  <span className="graph-group-icon">
                    <img src={RESOURCE_GROUP_ICON} alt="" />
                  </span>
                  <span className="graph-group-copy">
                    <strong>{node.name}</strong>
                    <small>{node.subtitle}</small>
                  </span>
                  {node.findingCount > 0 ? <em>{node.findingCount}</em> : null}
                </span>
                <span className="graph-group-types" aria-hidden="true">
                  <span className="graph-group-open">Select group</span>
                </span>
              </button>
            );
          }
          if (node.kind === "resource") {
            const selected = node.id === focusedNodeId;
            const dim = (node.hop ?? 0) > 1;
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className={[
                  "graph-node-label",
                  selected ? "selected" : "",
                  dim ? "dimmed" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onClick={() => onSelectResource(node.resourceId ?? node.id)}
                onFocus={() => cyRef.current?.getElementById(node.id).addClass("keyboard-focus")}
                onBlur={() => cyRef.current?.getElementById(node.id).removeClass("keyboard-focus")}
                aria-label={`${node.name}, ${typeName(node.azureType)}`}
              >
                <span className="graph-node-icon">
                  {typeIcon(node.azureType) ? <img src={typeIcon(node.azureType)} alt="" /> : null}
                </span>
                <span className="graph-node-copy">
                  <strong>{node.name}</strong>
                  <small>{node.subtitle || typeName(node.azureType)}</small>
                </span>
                {node.findingCount > 0 ? <em>{node.findingCount}</em> : null}
              </button>
            );
          }
          return null;
        })}
      </div>
      <div className="graph-controls-hint" aria-hidden="true">
        <span>
          {graph.level === "estate"
            ? "Select a group · Enter to open · click a lane bar to expand"
            : graph.level === "group"
              ? "Drag nodes · aggregated tiles list their members"
              : "Drag nodes"}
        </span>
        <i /> <span>Drag canvas to pan</span>
        <i /> <span>Scroll to zoom</span>
      </div>
      {graph.nodes.length === 0 ? (
        <div className="graph-empty-state" role="status">
          <img src={RESOURCE_GROUP_ICON} alt="" />
          <strong>Nothing to draw</strong>
          <span>The current scope contains no stored resources.</span>
        </div>
      ) : null}
      {rendererError ? (
        <div className="graph-renderer-error" role="status">
          <strong>Graph rendering is unavailable</strong>
          <span>{rendererError}</span>
          <p>The resource list remains available in the relationship inspector.</p>
        </div>
      ) : null}
    </div>
  );
}
