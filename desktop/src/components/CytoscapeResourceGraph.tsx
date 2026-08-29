import { useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import cytoscape, { type Core, type ElementDefinition, type StylesheetJson } from "cytoscape";
import { GitBranch } from "lucide-react";
import { RESOURCE_GROUP_ICON, SUBSCRIPTION_ICON, VNET_ICON } from "../azure-icons";
import type { EstateSnapshot, TopologyGraph, TopologyNode } from "../types";
import {
  GRAPH_SIZE,
  cameraEntryZoom,
  cameraProfile,
  cameraTargetIds,
  layoutTopology,
  nearestNodeInDirection,
  type Placement,
  type SpatialDirection,
  type TopologyLayoutPlan,
} from "./topology-layout";

export type GraphMode = "neighbourhood" | "estate";

export type GraphCameraMode = "core" | "selection" | "all" | "zoom-in" | "zoom-out";

export interface GraphCameraRequest {
  mode: GraphCameraMode;
  nonce: number;
}

export type GraphActivation =
  | { kind: "resource"; resourceId: string }
  | { kind: "resource-neighbourhood"; resourceId: string }
  | { kind: "resource-group"; groupId: string }
  | { kind: "aggregate"; nodeId: string }
  | { kind: "subscription"; subscriptionId: string };

interface CytoscapeResourceGraphProps {
  graph: TopologyGraph;
  estate: EstateSnapshot;
  theme: "light" | "dark";
  selectedNodeId?: string;
  expandedAggregateId?: string;
  motionEnabled: boolean;
  camera: GraphCameraRequest;
  onActivate: (activation: GraphActivation) => void;
}

const RESOURCE_W = GRAPH_SIZE.resourceWidth;
const RESOURCE_H = GRAPH_SIZE.resourceHeight;
const GROUP_W = GRAPH_SIZE.groupWidth;
const GROUP_H = GRAPH_SIZE.groupHeight;
const AGGREGATE_W = GRAPH_SIZE.aggregateWidth;
const AGGREGATE_H = GRAPH_SIZE.aggregateHeight;
const LANE_BAR_W = GRAPH_SIZE.laneBarWidth;
const LANE_BAR_H = GRAPH_SIZE.laneBarHeight;

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
      selector: "node.selected",
      style: {
        "background-color": palette.focusFill,
        "background-opacity": 1,
        "border-width": 1,
        "border-color": palette.nodeBorder,
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
        label: "data(label)",
        color: palette.labelText,
        "font-family": "Plex Mono, monospace",
        "font-size": 12.5,
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

const LABEL_SCALE_FLOOR = 0.88;

function nodeClasses(node: TopologyNode, selectedNodeId?: string) {
  const classes: string[] = [];
  if (node.kind === "resource") classes.push("resource-node");
  if (node.kind === "resource-group") classes.push("resource-group-node");
  if (node.kind === "aggregate") classes.push("aggregate-node");
  if (node.kind === "external") classes.push("external-node");
  if (node.kind === "subscription") classes.push("lane-bar-node");
  if (node.kind === "vnet") classes.push("vnet-frame");
  if (node.kind === "subnet") classes.push("subnet-frame");
  if ((node.hop ?? 0) > 1) classes.push("hop-dim");
  if (node.id === selectedNodeId) classes.push("selected");
  return classes.join(" ");
}

function graphElements(graph: TopologyGraph, selectedNodeId?: string): ElementDefinition[] {
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
      classes: isEmptySubnet ? "subnet-empty" : nodeClasses(node, selectedNodeId),
      selectable: node.kind !== "vnet" && node.kind !== "subnet",
      grabbable: node.kind === "resource" || node.kind === "aggregate" || node.kind === "external",
    });
  }
  graph.links.forEach((link, index) => {
    const related = Boolean(selectedNodeId && (link.sourceId === selectedNodeId || link.targetId === selectedNodeId));
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
  selectedNodeId,
  expandedAggregateId,
  motionEnabled,
  camera,
  onActivate,
}: CytoscapeResourceGraphProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const surfaceRef = useRef<HTMLDivElement>(null);
  const labelRefs = useRef(new Map<string, HTMLElement>());
  const cyRef = useRef<Core | undefined>(undefined);
  const activateRef = useRef(onActivate);
  const motionRef = useRef(motionEnabled);
  const selectedNodeRef = useRef(selectedNodeId);
  const cameraRef = useRef(camera);
  const layoutPlanRef = useRef<TopologyLayoutPlan | undefined>(undefined);
  const cameraApiRef = useRef<((mode: GraphCameraMode, animate: boolean) => void) | undefined>(undefined);
  const activationApiRef = useRef<((activation: GraphActivation, nodeId?: string) => void) | undefined>(undefined);
  const labelSyncRef = useRef<(() => void) | undefined>(undefined);
  const motionSchedulerRef = useRef<(() => void) | undefined>(undefined);
  const [rendererError, setRendererError] = useState<string>();
  const [rendererRetryNonce, setRendererRetryNonce] = useState(0);
  const [keyboardNodeId, setKeyboardNodeId] = useState<string>();
  const typeMap = useMemo(
    () => new Map(estate.resourceTypes.map((type) => [type.azureType, type])),
    [estate.resourceTypes],
  );
  const resourceMap = useMemo(
    () => new Map(estate.resources.map((resource) => [resource.id, resource])),
    [estate.resources],
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
  const selectedSummary = useMemo(() => {
    if (!selectedNodeId) return "No graph item selected.";
    const selected = graph.nodes.find((node) => node.id === selectedNodeId);
    if (!selected) return "The selected item is outside the current graph scope.";
    const links = graph.links.filter((link) => link.sourceId === selectedNodeId || link.targetId === selectedNodeId);
    if (links.length === 0) return `${selected.name}. No drawn connectors in this scope.`;
    const names = new Map(graph.nodes.map((node) => [node.id, node.name]));
    const descriptions = links.map((link) => {
      const outbound = link.sourceId === selectedNodeId;
      const other = names.get(outbound ? link.targetId : link.sourceId) ?? "another represented item";
      return `${outbound ? "outbound" : "inbound"} ${link.label} ${outbound ? "to" : "from"} ${other}`;
    });
    return `${selected.name}. ${links.length} connector${links.length === 1 ? "" : "s"}: ${descriptions.join("; ")}.`;
  }, [graph.links, graph.nodes, selectedNodeId]);

  useEffect(() => {
    activateRef.current = onActivate;
  }, [onActivate]);

  useEffect(() => {
    motionRef.current = motionEnabled;
    motionSchedulerRef.current?.();
  }, [motionEnabled]);

  useEffect(() => {
    selectedNodeRef.current = selectedNodeId;
    const cy = cyRef.current;
    if (!cy) return;
    cy.batch(() => {
      cy.nodes().removeClass("selected");
      if (selectedNodeId) cy.getElementById(selectedNodeId).addClass("selected");
      cy.edges(".relationship-edge").forEach((edge) => {
        const related = Boolean(
          selectedNodeId && (edge.source().id() === selectedNodeId || edge.target().id() === selectedNodeId),
        );
        edge.toggleClass("related", related);
      });
    });
    if (selectedNodeId) setKeyboardNodeId(selectedNodeId);
    labelSyncRef.current?.();
  }, [selectedNodeId]);

  useEffect(() => {
    if (camera.mode !== "zoom-in" && camera.mode !== "zoom-out") cameraRef.current = camera;
    cameraApiRef.current?.(camera.mode, camera.nonce > 0);
  }, [camera]);

  useEffect(() => {
    setKeyboardNodeId((current) => {
      if (current && graph.nodes.some((node) => node.id === current)) return current;
      if (selectedNodeId && graph.nodes.some((node) => node.id === selectedNodeId)) return selectedNodeId;
      return graph.nodes.find((node) => !["vnet", "subnet"].includes(node.kind))?.id;
    });
  }, [graph.nodes, selectedNodeId]);

  useEffect(() => {
    const surface = surfaceRef.current;
    const host = hostRef.current;
    if (!surface || !host || graph.nodes.length === 0) return;
    const activeHost = host;

    let cy: Core;
    try {
      cy = cytoscape({
        container: surface,
        elements: graphElements(graph, selectedNodeRef.current),
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
      activeHost.classList.toggle("graph-zoom-compact", zoom < 0.9 && zoom >= 0.72);
      activeHost.classList.toggle("graph-zoom-overview", zoom < 0.72);
      const labelScale = Math.max(zoom, LABEL_SCALE_FLOOR);
      for (const [id, label] of labelRefs.current) {
        const node = cy.getElementById(id);
        if (node.empty()) continue;
        if (frameIds.has(id) && !node.hasClass("subnet-empty")) {
          const box = node.renderedBoundingBox({ includeLabels: false, includeOverlays: false });
          label.style.transform = `translate3d(${box.x1 + 14 * zoom}px, ${box.y1 + 6 * zoom}px, 0) scale(${labelScale})`;
        } else {
          const point = node.renderedPosition();
          label.style.transform = `translate3d(${point.x}px, ${point.y}px, 0) scale(${labelScale}) translate(-50%, -50%)`;
        }
      }
      const edgeFontSize = Math.max(12.5, 11 / Math.max(zoom, 0.1));
      cy.edges(".relationship-edge.related").style("font-size", edgeFontSize);
    }

    function queueLabelSync() {
      if (labelFrame !== 0) return;
      labelFrame = window.requestAnimationFrame(syncLabels);
    }

    function shouldAnimate() {
      return visible && !reduceMotion.matches && motionRef.current;
    }

    function scheduleFlow() {
      if (motionFrame !== 0 || !shouldAnimate()) return;
      motionFrame = window.requestAnimationFrame(animateFlow);
    }

    function animateFlow(time: number) {
      motionFrame = 0;
      if (!shouldAnimate()) return;
      if (time - lastMotionPaint < 38) {
        scheduleFlow();
        return;
      }
      lastMotionPaint = time;
      dashOffset = (dashOffset - 0.7) % 12;
      cy.edges(".relationship-edge.related").style("line-dash-offset", dashOffset);
      scheduleFlow();
    }

    function handleVisibility() {
      visible = document.visibilityState === "visible";
      scheduleFlow();
    }

    function handleMotionPreference() {
      scheduleFlow();
    }

    function handleTap(event: cytoscape.EventObject) {
      const kind = event.target.data("kind");
      if (kind === "resource-group") {
        activationApiRef.current?.(
          { kind: "resource-group", groupId: event.target.data("groupId") },
          event.target.id(),
        );
        return;
      }
      if (kind === "subscription") {
        activateRef.current({ kind: "subscription", subscriptionId: event.target.data("lane") });
        return;
      }
      if (kind === "aggregate") {
        activateRef.current({ kind: "aggregate", nodeId: event.target.id() });
        return;
      }
      if (kind === "external") {
        const resourceId = event.target.data("resourceId");
        if (resourceId) activationApiRef.current?.({ kind: "resource", resourceId }, event.target.id());
        else activateRef.current({ kind: "aggregate", nodeId: event.target.id() });
        return;
      }
      if (kind === "resource" || kind === "vnet") {
        const resourceId = event.target.data("resourceId");
        if (resourceId) activationApiRef.current?.({ kind: "resource", resourceId }, event.target.id());
      }
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
    reduceMotion.addEventListener("change", handleMotionPreference);

    // Until the user pans or zooms themselves, keep the graph fitted and
    // centred through container resizes — the stage often settles its final
    // size a frame after the graph mounts.
    let userAdjusted = false;
    const markUserAdjusted = () => {
      userAdjusted = true;
    };
    surface.addEventListener("pointerdown", markUserAdjusted);
    surface.addEventListener("wheel", markUserAdjusted, { passive: true });

    function collectionFor(ids: string[]) {
      let collection = cy.collection();
      for (const id of ids) {
        const node = cy.getElementById(id);
        if (node.nonempty()) collection = collection.union(node);
      }
      return collection;
    }

    function viewportAtZoom(target: cytoscape.CollectionReturnValue, zoom: number) {
      const bounds = target.boundingBox({ includeLabels: false, includeOverlays: false });
      return {
        zoom,
        pan: {
          x: (cy.width() - zoom * (bounds.x1 + bounds.x2)) / 2,
          y: (cy.height() - zoom * (bounds.y1 + bounds.y2)) / 2,
        },
      };
    }

    function resolvedCamera(mode: Exclude<GraphCameraMode, "zoom-in" | "zoom-out">) {
      const plan = layoutPlanRef.current;
      if (!plan) return undefined;
      const selectedId = selectedNodeRef.current;
      const targetIds = cameraTargetIds(plan, cy.nodes().map((node) => node.id()), mode, selectedId);
      const requested = collectionFor(targetIds);
      const target = requested.nonempty() ? requested : cy.elements();
      const profile = cameraProfile(
        graph.level,
        mode,
        targetIds.length,
        { width: activeHost.clientWidth, height: activeHost.clientHeight },
      );
      const getFitViewport = cy.getFitViewport as unknown as (
        elements: cytoscape.CollectionReturnValue,
        padding: number,
      ) => { zoom: number; pan: cytoscape.Position } | undefined;
      const fitted = getFitViewport.call(cy, target, profile.padding);
      if (!fitted) return undefined;
      const zoom = Math.min(profile.maxZoom, Math.max(profile.minZoom, fitted.zoom));
      return { target, profile, viewport: viewportAtZoom(target, zoom) };
    }

    let resizePending = false;

    function applyCamera(mode: GraphCameraMode, animate: boolean) {
      if (mode === "zoom-in" || mode === "zoom-out") {
        const zoom = {
          level: Math.min(2.2, Math.max(0.1, cy.zoom() * (mode === "zoom-in" ? 1.22 : 0.82))),
          renderedPosition: { x: activeHost.clientWidth / 2, y: activeHost.clientHeight / 2 },
        };
        cy.stop();
        if (animate && !reduceMotion.matches) {
          cy.animate({ zoom }, { duration: 180, easing: "ease-out-cubic", complete: queueLabelSync });
        } else {
          cy.zoom(zoom);
          queueLabelSync();
        }
        return;
      }
      const resolved = resolvedCamera(mode);
      if (!resolved) return;
      const finish = () => {
        queueLabelSync();
        if (!resizePending) return;
        resizePending = false;
        applyLayout();
        applyCamera(cameraRef.current.mode, false);
      };
      cy.stop();
      if (animate && !reduceMotion.matches) {
        cy.animate(
          { zoom: resolved.viewport.zoom, pan: resolved.viewport.pan },
          { duration: resolved.profile.duration, easing: "ease-out-cubic", complete: finish },
        );
      } else {
        cy.viewport(resolved.viewport);
        finish();
      }
    }

    let activationPending = false;
    function activateWithCamera(activation: GraphActivation, nodeId?: string) {
      const navigates = activation.kind === "resource"
        || activation.kind === "resource-neighbourhood"
        || activation.kind === "resource-group";
      const node = nodeId ? cy.getElementById(nodeId) : cy.collection();
      if (!navigates || node.empty() || reduceMotion.matches) {
        activateRef.current(activation);
        return;
      }
      if (activationPending) return;
      activationPending = true;
      const zoom = Math.min(1.72, Math.max(cy.zoom() + 0.14, cy.zoom() * 1.16));
      const viewport = viewportAtZoom(node, zoom);
      cy.stop();
      cy.animate(
        { zoom: viewport.zoom, pan: viewport.pan },
        {
          duration: 180,
          easing: "ease-out-cubic",
          complete: () => {
            activationPending = false;
            activateRef.current(activation);
          },
        },
      );
    }

    function applyLayout() {
      const plan = layoutTopology(graph, {
        width: activeHost.clientWidth,
        height: activeHost.clientHeight,
      });
      layoutPlanRef.current = plan;
      cy.batch(() => {
        for (const [id, placement] of plan.positions) {
          const node = cy.getElementById(id);
          if (!node.empty() && !node.isParent()) node.position({ x: placement.x, y: placement.y });
        }
      });
    }

    try {
      applyLayout();
      const entryMode = cameraRef.current.mode;
      if (entryMode === "zoom-in" || entryMode === "zoom-out" || reduceMotion.matches) {
        applyCamera(entryMode, false);
      } else {
        const resolved = resolvedCamera(entryMode);
        if (resolved) {
          cy.viewport(viewportAtZoom(resolved.target, cameraEntryZoom(graph.level, resolved.viewport.zoom)));
          applyCamera(entryMode, true);
        }
      }
    } catch (error) {
      setRendererError(error instanceof Error ? error.message : "The relationship layout could not be calculated.");
      cyRef.current = undefined;
      cy.destroy();
      return;
    }

    let previousHostSize = {
      width: activeHost.clientWidth,
      height: activeHost.clientHeight,
    };
    const resizeObserver = new ResizeObserver(() => {
      const nextHostSize = {
        width: activeHost.clientWidth,
        height: activeHost.clientHeight,
      };
      const viewportChanged = Math.abs(nextHostSize.width - previousHostSize.width) >= 48
        || Math.abs(nextHostSize.height - previousHostSize.height) >= 48;
      previousHostSize = nextHostSize;
      if (viewportChanged) userAdjusted = false;
      cy.resize();
      window.cancelAnimationFrame(resizeFrame);
      resizeFrame = window.requestAnimationFrame(() => {
        resizeFrame = 0;
        if (!userAdjusted && cy.animated()) {
          resizePending = true;
        } else if (!userAdjusted) {
          applyLayout();
          applyCamera(cameraRef.current.mode, false);
        }
        queueLabelSync();
      });
    });
    resizeObserver.observe(activeHost);
    cameraApiRef.current = applyCamera;
    activationApiRef.current = activateWithCamera;
    labelSyncRef.current = queueLabelSync;
    motionSchedulerRef.current = scheduleFlow;
    scheduleFlow();
    queueLabelSync();

    return () => {
      window.cancelAnimationFrame(labelFrame);
      window.cancelAnimationFrame(resizeFrame);
      window.cancelAnimationFrame(motionFrame);
      resizeObserver.disconnect();
      surface.removeEventListener("pointerdown", markUserAdjusted);
      surface.removeEventListener("wheel", markUserAdjusted);
      document.removeEventListener("visibilitychange", handleVisibility);
      reduceMotion.removeEventListener("change", handleMotionPreference);
      cameraApiRef.current = undefined;
      activationApiRef.current = undefined;
      labelSyncRef.current = undefined;
      motionSchedulerRef.current = undefined;
      layoutPlanRef.current = undefined;
      cyRef.current = undefined;
      cy.destroy();
    };
    // The graph rebuilds when its structure OR the resolved theme changes —
    // the palette and per-edge colours are read from CSS tokens at build time.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graphStructureKey, rendererRetryNonce, theme]);

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

  function findingText(node: TopologyNode) {
    return node.findingCount > 0
      ? `, ${node.findingCount} finding${node.findingCount === 1 ? "" : "s"}`
      : "";
  }

  function spatialPoints() {
    const points = new Map<string, Placement>();
    for (const [id, element] of labelRefs.current) {
      const button = element instanceof HTMLButtonElement
        ? element
        : element.querySelector<HTMLButtonElement>("[data-graph-roving]");
      if (!button || button.disabled) continue;
      const rect = button.getBoundingClientRect();
      points.set(id, { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 });
    }
    return points;
  }

  function handleGraphButtonKeyDown(id: string, event: ReactKeyboardEvent<HTMLButtonElement>) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      event.currentTarget.click();
      return;
    }
    if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      cameraApiRef.current?.("zoom-in", true);
      return;
    }
    if (event.key === "-") {
      event.preventDefault();
      cameraApiRef.current?.("zoom-out", true);
      return;
    }
    if (event.key === "0") {
      event.preventDefault();
      cameraApiRef.current?.("core", true);
      return;
    }
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) return;
    event.preventDefault();
    const next = nearestNodeInDirection(id, event.key as SpatialDirection, spatialPoints());
    if (!next) return;
    setKeyboardNodeId(next);
    window.requestAnimationFrame(() => {
      const element = labelRefs.current.get(next);
      if (element instanceof HTMLButtonElement) element.focus();
      else element?.querySelector<HTMLButtonElement>("[data-graph-roving]")?.focus();
    });
  }

  function graphButtonProps(id: string) {
    return {
      tabIndex: keyboardNodeId === id ? 0 : -1,
      onFocus: () => {
        setKeyboardNodeId(id);
        cyRef.current?.getElementById(id).addClass("keyboard-focus");
      },
      onBlur: () => cyRef.current?.getElementById(id).removeClass("keyboard-focus"),
      onKeyDown: (event: ReactKeyboardEvent<HTMLButtonElement>) => handleGraphButtonKeyDown(id, event),
      "data-graph-roving": true,
    };
  }

  function resourceButtonProps(id: string, resourceId: string, relationshipCount: number) {
    const props = graphButtonProps(id);
    return {
      ...props,
      "aria-keyshortcuts": "R",
      onKeyDown: (event: ReactKeyboardEvent<HTMLButtonElement>) => {
        if (relationshipCount > 0
          && event.key.toLowerCase() === "r"
          && !event.metaKey
          && !event.ctrlKey
          && !event.altKey) {
          event.preventDefault();
          requestActivation({ kind: "resource-neighbourhood", resourceId }, id);
          return;
        }
        props.onKeyDown(event);
      },
    };
  }

  function requestActivation(activation: GraphActivation, nodeId?: string) {
    const activate = activationApiRef.current;
    if (activate) activate(activation, nodeId);
    else activateRef.current(activation);
  }

  return (
    <div className="cytoscape-graph" ref={hostRef}>
      <div className="graph-aurora" aria-hidden="true" />
      <div className="cytoscape-surface" ref={surfaceRef} aria-hidden="true" />
      <div className="graph-label-layer">
        {graph.lanes
          .filter((lane) => lane.expanded && graph.nodes.some((node) => node.kind === "resource-group" && node.lane === lane.subscriptionId))
          .map((lane) => (
            <button
              key={`lane:${lane.subscriptionId}`}
              ref={registerLabel(`lane:${lane.subscriptionId}`)}
              className="graph-container-label graph-lane-label"
              onClick={() => onActivate({ kind: "subscription", subscriptionId: lane.subscriptionId })}
              aria-label={`${lane.name}, expanded subscription, ${lane.groupCount} groups. Collapse subscription.`}
              {...graphButtonProps(`lane:${lane.subscriptionId}`)}
            >
              <img src={SUBSCRIPTION_ICON} alt="" />
              <strong>{lane.name}</strong>
              <small>
                {lane.groupCount} group{lane.groupCount === 1 ? "" : "s"} · {lane.resourceCount} resources
              </small>
              <i>Collapse</i>
            </button>
          ))}
        {graph.nodes.map((node) => {
          if (node.kind === "vnet") {
            const content = <>
                <img src={VNET_ICON} alt="" />
                <strong>{node.name}</strong>
                <small>{node.subtitle}</small>
              </>;
            return node.resourceId ? (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className="graph-container-label graph-vnet-label"
                onClick={() => requestActivation(
                  { kind: "resource", resourceId: node.resourceId ?? node.id },
                  node.id,
                )}
                aria-pressed={node.id === selectedNodeId}
                aria-label={`${node.name}, virtual network${findingText(node)}. Open resource record.`}
                {...graphButtonProps(node.id)}
              >{content}</button>
            ) : (
              <span key={node.id} ref={registerLabel(node.id)} className="graph-container-label graph-vnet-label">{content}</span>
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
                onClick={() => onActivate({ kind: "subscription", subscriptionId: node.lane ?? "" })}
                aria-label={`${node.name}, collapsed subscription, ${node.count} groups${findingText(node)}. Expand subscription.`}
                {...graphButtonProps(node.id)}
              >
                <img src={SUBSCRIPTION_ICON} alt="" />
                <span className="graph-node-copy">
                  <strong>{node.name}</strong>
                  <small>{node.subtitle}</small>
                </span>
                {node.findingCount > 0 ? <em aria-label={`${node.findingCount} findings`}>{node.findingCount}</em> : null}
                <i className="graph-lane-expand">Expand ›</i>
              </button>
            );
          }
          if (node.kind === "aggregate") {
            const expanded = node.id === expandedAggregateId;
            const members = node.memberIds
              .map((id) => resourceMap.get(id))
              .filter((resource): resource is NonNullable<typeof resource> => Boolean(resource));
            return (
              <div
                key={node.id}
                ref={registerLabel(node.id)}
                className={expanded ? "graph-aggregate-cluster expanded" : "graph-aggregate-cluster"}
                onKeyDown={(event) => {
                  if (event.key !== "Escape" || !expanded) return;
                  event.preventDefault();
                  onActivate({ kind: "aggregate", nodeId: node.id });
                  event.currentTarget.querySelector<HTMLButtonElement>("[data-graph-roving]")?.focus();
                }}
              >
                <button
                  className={expanded ? "graph-aggregate-label selected" : "graph-aggregate-label"}
                  onClick={() => onActivate({ kind: "aggregate", nodeId: node.id })}
                  aria-expanded={expanded}
                  aria-label={`${node.count} ${typeName(node.azureType)}, aggregated${findingText(node)}. ${expanded ? "Collapse" : "Show"} resources.`}
                  {...graphButtonProps(node.id)}
                >
                  {typeIcon(node.azureType) ? <img src={typeIcon(node.azureType)} alt="" /> : null}
                  <span className="graph-node-copy">
                    <strong>{typeName(node.azureType)}</strong>
                    <small>{node.zone === "unconnected" ? "No drawn connectors" : `${node.hop ?? 1} hop${(node.hop ?? 1) === 1 ? "" : "s"} away`}</small>
                  </span>
                  <b>{node.subtitle}</b>
                  {node.findingCount > 0 ? <em aria-label={`${node.findingCount} findings`}>{node.findingCount}</em> : null}
                </button>
                {expanded ? (
                  <div className="graph-aggregate-members" role="group" aria-label={`${typeName(node.azureType)} resources`}>
                    {members.map((resource) => (
                      <button key={resource.id} onClick={() => requestActivation(
                        { kind: "resource", resourceId: resource.id },
                        node.id,
                      )} title={resource.name}>
                        {typeIcon(resource.azureType) ? <img src={typeIcon(resource.azureType)} alt="" /> : null}
                        <span><strong>{resource.name}</strong><small>{resource.findingCount > 0 ? `${resource.findingCount} findings` : "Open resource record"}</small></span>
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            );
          }
          if (node.kind === "external") {
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className={node.id === selectedNodeId ? "graph-node-label ghost selected" : "graph-node-label ghost"}
                onClick={() => node.resourceId
                  ? requestActivation({ kind: "resource", resourceId: node.resourceId }, node.id)
                  : onActivate({ kind: "aggregate", nodeId: node.id })}
                aria-pressed={node.id === selectedNodeId}
                aria-label={`${node.name}, in another resource group${findingText(node)}. Open resource record.`}
                {...graphButtonProps(node.id)}
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
            const selected = node.id === selectedNodeId;
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className={selected ? "graph-resource-group-label selected" : "graph-resource-group-label"}
                onClick={() => requestActivation(
                  { kind: "resource-group", groupId: node.groupId ?? "" },
                  node.id,
                )}
                aria-pressed={selected}
                aria-label={`${node.name}, ${node.count} resources${findingText(node)}. Open group map.`}
                {...graphButtonProps(node.id)}
              >
                <span className="graph-group-heading">
                  <span className="graph-group-icon">
                    <img src={RESOURCE_GROUP_ICON} alt="" />
                  </span>
                  <span className="graph-group-copy">
                    <strong>{node.name}</strong>
                    <small>{node.subtitle}</small>
                  </span>
                  {node.findingCount > 0 ? <em aria-label={`${node.findingCount} findings`}>{node.findingCount}</em> : null}
                </span>
                <span className="graph-group-types" aria-hidden="true">
                  <span className="graph-group-open">Open group ›</span>
                </span>
              </button>
            );
          }
          if (node.kind === "resource") {
            const selected = node.id === selectedNodeId;
            const dim = (node.hop ?? 0) > 1;
            const resourceId = node.resourceId ?? node.id;
            const relationshipCount = resourceMap.get(resourceId)?.edgeCount ?? 0;
            return (
              <div
                key={node.id}
                ref={registerLabel(node.id)}
                className={[
                  "graph-node-label",
                  "graph-node-actions",
                  selected ? "selected" : "",
                  dim ? "dimmed" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                title={node.name}
              >
                <button
                  className="graph-node-primary"
                  onClick={() => requestActivation({ kind: "resource", resourceId }, node.id)}
                  aria-pressed={selected}
                  aria-label={`${node.name}, ${typeName(node.azureType)}${findingText(node)}. Open resource record.${relationshipCount > 0 ? ` Press R to explore ${relationshipCount} relationships.` : " No relationships to explore."}`}
                  {...resourceButtonProps(node.id, resourceId, relationshipCount)}
                >
                  <span className="graph-node-icon">
                    {typeIcon(node.azureType) ? <img src={typeIcon(node.azureType)} alt="" /> : null}
                  </span>
                  <span className="graph-node-copy">
                    <strong>{node.name}</strong>
                    <small>{node.subtitle || typeName(node.azureType)}</small>
                  </span>
                  {node.findingCount > 0 ? <em aria-label={`${node.findingCount} findings`}>{node.findingCount}</em> : null}
                </button>
                <button
                  className="graph-node-relationships"
                  tabIndex={-1}
                  disabled={relationshipCount === 0}
                  onClick={() => requestActivation(
                    { kind: "resource-neighbourhood", resourceId },
                    node.id,
                  )}
                  aria-label={relationshipCount > 0
                    ? `Explore ${relationshipCount} relationships for ${node.name}`
                    : `${node.name} has no relationships to explore`}
                  title={relationshipCount > 0
                    ? `Explore ${relationshipCount} relationships`
                    : "No relationships to explore"}
                >
                  <GitBranch size={13} aria-hidden="true" />
                  <span>{relationshipCount}</span>
                </button>
              </div>
            );
          }
          return null;
        })}
      </div>
      <div className="sr-only" aria-live="polite">{selectedSummary}</div>
      {graph.nodes.length === 0 ? (
        <div className="graph-empty-state" role="status">
          <img src={RESOURCE_GROUP_ICON} alt="" />
          <strong>Nothing to draw</strong>
          <span>The current scope contains no stored resources.</span>
        </div>
      ) : null}
      {rendererError ? (
        <div className="graph-renderer-error" role="alert" aria-live="assertive">
          <strong>Graph rendering is unavailable</strong>
          <span>{rendererError}</span>
          <p>The relationship view can be retried without leaving this page.</p>
          <button onClick={() => setRendererRetryNonce((value) => value + 1)}>Retry renderer</button>
        </div>
      ) : null}
    </div>
  );
}
