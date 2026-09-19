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
import {
  activeRelationshipLabels,
  buildLinkPresentations,
  connectorPortAssignments,
  connectorPortPosition,
  placeRelationshipLabel,
  resolveTraceNode,
  type ActiveRelationshipLabel,
  type LabelRect,
  type LinkPresentation,
} from "./topology-presentation";
import { EmptyState } from "./view-chrome";
import { useResourceTypeMap } from "../estate-lookups";
import { errorMessage, fill, plural } from "../format";
import { labels, useLabels } from "../labels";
import { cssToken, kindClassColor } from "./graph-tokens";

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

interface GraphPalette {
  nodeFill: string;
  nodeBorder: string;
  frameBorder: string;
  focusFill: string;
  focusBorder: string;
}

function readGraphPalette(): GraphPalette {
  return {
    nodeFill: cssToken("--graph-node-fill", "#ffffff"),
    nodeBorder: cssToken("--graph-node-border", "#d8d2c6"),
    frameBorder: cssToken("--graph-frame-border", "#9aa2ac"),
    focusFill: cssToken("--graph-focus-fill", "#f3efe7"),
    focusBorder: cssToken("--graph-focus-border", "#0b5da8"),
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
        "z-index-compare": "manual",
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
        "z-index-compare": "manual",
      },
    },
    {
      // The estate tile sits in a group-card cell and carries a longer label.
      selector: "node.group-aggregate-node",
      style: { width: GROUP_W },
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
        "z-index-compare": "manual",
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
        "z-index-compare": "manual",
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
        "z-index-compare": "manual",
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
        "z-index-compare": "manual",
        // Cytoscape aliases every `padding-<side>` to `padding`, so a taller
        // top band cannot be asked for here; the DOM header sits on the
        // frame's top border instead (see syncLabels).
        padding: "22px",
      },
    },
    {
      selector: "node.subnet-frame",
      style: {
        "corner-radius": "4px",
        "border-opacity": 0.6,
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
        "z-index-compare": "manual",
      },
    },
    {
      selector: "node.hop-dim",
      style: { "background-opacity": 0.55, "border-opacity": 0.55 },
    },
    {
      selector: "node.resource-node.selected",
      style: {
        width: 220,
        height: 92,
      },
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
      selector: "node.trace-source, node.trace-peer",
      style: {
        "background-opacity": 1,
        "border-opacity": 1,
      },
    },
    {
      selector: "node.trace-muted",
      style: {
        "background-opacity": 0.34,
        "border-opacity": 0.34,
      },
    },
    {
      selector: "node.connector-port",
      style: {
        width: 2,
        height: 2,
        "background-opacity": 0,
        "border-width": 0,
        opacity: 0,
        events: "no",
        label: "",
        "z-index": 0,
        "z-index-compare": "manual",
      },
    },
    {
      selector: "edge.relationship-edge",
      style: {
        width: "mapData(weight, 1, 20, 1.2, 3)",
        "curve-style": "round-taxi",
        "taxi-direction": "horizontal",
        "taxi-turn": "data(taxiTurn)",
        "taxi-turn-min-distance": "28px",
        "taxi-radius": 9,
        "source-endpoint": "outside-to-node",
        "target-endpoint": "outside-to-node",
        "edge-distances": "intersection",
        "line-color": "data(color)",
        "line-opacity": 0.48,
        "line-cap": "round",
        "target-arrow-shape": "triangle",
        "target-arrow-color": "data(color)",
        "arrow-scale": 0.66,
        "source-distance-from-node": "0px",
        "target-distance-from-node": "0px",
        "overlay-opacity": 0,
        "underlay-opacity": 0,
        events: "no",
        "z-index": 2,
        "z-index-compare": "manual",
      },
    },
    {
      selector: "edge.relationship-edge.trace-muted",
      style: {
        "line-opacity": 0.13,
        "target-arrow-shape": "none",
      },
    },
    {
      selector: "edge.relationship-edge.trace-active",
      style: {
        width: "mapData(weight, 1, 20, 2.1, 4.6)",
        "line-opacity": 1,
        "line-style": "dashed",
        "line-dash-pattern": [7, 5],
        "underlay-color": "data(color)",
        "underlay-opacity": 0.14,
        "underlay-padding": "5px",
        "z-index": 7,
      },
    },
  ];
}

const LABEL_SCALE_FLOOR = 0.88;

/** Estate-level nodes that live inside a subscription lane frame. */
function laneMember(node: TopologyNode) {
  return Boolean(node.lane) && (node.kind === "resource-group" || (node.kind === "aggregate" && node.groupIds.length > 0));
}

function nodeClasses(node: TopologyNode, selectedNodeId?: string) {
  const classes: string[] = [];
  if (node.kind === "resource") classes.push("resource-node");
  if (node.kind === "resource-group") classes.push("resource-group-node");
  if (node.kind === "aggregate") classes.push("aggregate-node");
  if (node.kind === "aggregate" && node.groupIds.length > 0) classes.push("group-aggregate-node");
  if (node.kind === "external") classes.push("external-node");
  if (node.kind === "subscription") classes.push("lane-bar-node");
  if (node.kind === "vnet") classes.push("vnet-frame");
  if (node.kind === "subnet") classes.push("subnet-frame");
  if ((node.hop ?? 0) > 1) classes.push("hop-dim");
  if (node.id === selectedNodeId) classes.push("selected");
  return classes.join(" ");
}

function graphElements(
  graph: TopologyGraph,
  presentations: readonly LinkPresentation[],
  selectedNodeId?: string,
): ElementDefinition[] {
  const childCount = new Map<string, number>();
  for (const node of graph.nodes) {
    if (node.parentId) childCount.set(node.parentId, (childCount.get(node.parentId) ?? 0) + 1);
  }
  // A lane is framed when it holds anything: cards, or only the unconnected
  // groups tile. A subscription whose groups are all unconnected still needs
  // its name above the tile.
  const laneHasMembers = new Set(
    graph.nodes.filter((node) => laneMember(node)).map((node) => node.lane as string),
  );
  const elements: ElementDefinition[] = [];
  for (const lane of graph.lanes.filter((candidate) => candidate.expanded)) {
    if (!laneHasMembers.has(lane.subscriptionId)) continue;
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
      (graph.level === "estate" && laneMember(node) && laneHasMembers.has(node.lane as string)
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
    const presentation = presentations[index];
    elements.push(
      {
        group: "nodes",
        data: {
          id: presentation.sourcePortId,
          kind: "connector-port",
          logicalNodeId: link.sourceId,
        },
        classes: "connector-port",
        selectable: false,
        grabbable: false,
      },
      {
        group: "nodes",
        data: {
          id: presentation.targetPortId,
          kind: "connector-port",
          logicalNodeId: link.targetId,
        },
        classes: "connector-port",
        selectable: false,
        grabbable: false,
      },
    );
    elements.push({
      group: "edges",
      data: {
        id: presentation.edgeId,
        source: presentation.sourcePortId,
        target: presentation.targetPortId,
        logicalSource: link.sourceId,
        logicalTarget: link.targetId,
        weight: link.count,
        color: kindClassColor(link.kindClass),
        taxiTurn: presentation.taxiTurn,
      },
      classes: "relationship-edge",
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
  const relationshipLabelRefs = useRef(new Map<string, HTMLElement>());
  const regionLabelRefs = useRef(new Map<string, HTMLElement>());
  const cyRef = useRef<Core | undefined>(undefined);
  const activateRef = useRef(onActivate);
  const motionRef = useRef(motionEnabled);
  const selectedNodeRef = useRef(selectedNodeId);
  const cameraRef = useRef(camera);
  const layoutPlanRef = useRef<TopologyLayoutPlan | undefined>(undefined);
  const cameraApiRef = useRef<((mode: GraphCameraMode, animate: boolean) => void) | undefined>(undefined);
  const activationApiRef = useRef<((activation: GraphActivation, nodeId?: string) => void) | undefined>(undefined);
  const traceApiRef = useRef<((nodeId?: string) => void) | undefined>(undefined);
  const labelSyncRef = useRef<(() => void) | undefined>(undefined);
  const motionSchedulerRef = useRef<(() => void) | undefined>(undefined);
  const [rendererError, setRendererError] = useState<string>();
  const [rendererRetryNonce, setRendererRetryNonce] = useState(0);
  const [keyboardNodeId, setKeyboardNodeId] = useState<string>();
  const [pointerTraceNodeId, setPointerTraceNodeId] = useState<string>();
  const [focusedTraceNodeId, setFocusedTraceNodeId] = useState<string>();
  const typeMap = useResourceTypeMap(estate);
  const resourceMap = useMemo(
    () => new Map(estate.resources.map((resource) => [resource.id, resource])),
    [estate.resources],
  );
  const linkPresentations = useMemo(() => buildLinkPresentations(graph), [graph]);
  const activeTraceNodeId = resolveTraceNode(focusedTraceNodeId, pointerTraceNodeId, selectedNodeId);
  const activeTraceNodeRef = useRef(activeTraceNodeId);
  const activeLabels = useMemo(
    () => activeRelationshipLabels(linkPresentations, activeTraceNodeId),
    [activeTraceNodeId, linkPresentations],
  );
  const activePeerIds = useMemo(
    () => new Set(activeLabels.map((label) => label.endpointId)),
    [activeLabels],
  );
  const connectionMeta = useMemo(() => {
    const result = new Map<string, { connectors: number; relationships: number; kinds: Set<string> }>();
    for (const link of graph.links) {
      for (const id of new Set([link.sourceId, link.targetId])) {
        const current = result.get(id) ?? { connectors: 0, relationships: 0, kinds: new Set<string>() };
        current.connectors += 1;
        current.relationships += link.count;
        current.kinds.add(link.kindClass);
        result.set(id, current);
      }
    }
    return result;
  }, [graph.links]);
  const { graph: words, graph_extra: extra, regions: regionWords } = useLabels().desktop.topology;
  const graphRegions = useMemo(() => {
    if (graph.level !== "group") return [];
    const candidates = [
      {
        id: "external",
        label: regionWords.external,
        nodes: graph.nodes.filter((node) => node.zone === "external"),
      },
      {
        id: "services",
        label: regionWords.services,
        nodes: graph.nodes.filter(
          (node) => node.kind === "resource" && !node.parentId && node.zone === "core",
        ),
      },
      {
        id: "unconnected",
        label: regionWords.unconnected,
        nodes: graph.nodes.filter((node) => node.zone === "unconnected"),
      },
    ];
    return candidates
      .filter((region) => region.nodes.length > 0)
      .map((region) => ({
        id: region.id,
        label: region.label,
        count: region.nodes.reduce((total, node) => total + node.count, 0),
        nodeIds: region.nodes.map((node) => node.id),
      }));
  }, [graph, regionWords]);
  const graphStructureKey = useMemo(
    () =>
      [
        graph.level,
        graph.nodes.map((node) => `${node.id}~${node.parentId ?? ""}~${node.kind}~${node.zone ?? ""}~${node.hop ?? ""}`).join("|"),
        graph.links.map((link) => `${link.sourceId}~${link.targetId}~${link.kindClass}~${link.label}~${link.count}`).join("|"),
        graph.lanes.map((lane) => `${lane.subscriptionId}~${lane.expanded}`).join("|"),
      ].join("\n"),
    [graph],
  );
  const announcedTraceNodeId = focusedTraceNodeId ?? selectedNodeId;
  const selectedSummary = useMemo(() => {
    const text = labels().desktop.topology.graph;
    if (!announcedTraceNodeId) return text.summary_none;
    const selected = graph.nodes.find((node) => node.id === announcedTraceNodeId);
    if (!selected) return text.summary_out_of_scope;
    const links = graph.links.filter(
      (link) => link.sourceId === announcedTraceNodeId || link.targetId === announcedTraceNodeId,
    );
    if (links.length === 0) return fill(text.summary_no_connectors, { name: selected.name });
    const names = new Map(graph.nodes.map((node) => [node.id, node.name]));
    const descriptions = links.map((link) => {
      const outbound = link.sourceId === announcedTraceNodeId;
      const other = names.get(outbound ? link.targetId : link.sourceId) ?? text.other_item;
      return fill(text.connector_sentence, {
        direction: outbound ? text.outbound : text.inbound,
        label: link.label,
        preposition: outbound ? text.to : text.from,
        other,
      });
    });
    return fill(text.summary_connectors, {
      name: selected.name,
      connectors: plural(text.connectors, links.length),
      descriptions: descriptions.join("; "),
    });
  }, [announcedTraceNodeId, graph.links, graph.nodes]);

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
    });
    if (selectedNodeId) setKeyboardNodeId(selectedNodeId);
    labelSyncRef.current?.();
  }, [selectedNodeId]);

  useEffect(() => {
    activeTraceNodeRef.current = activeTraceNodeId;
    traceApiRef.current?.(activeTraceNodeId);
    labelSyncRef.current?.();
  }, [activeLabels, activeTraceNodeId]);

  useEffect(() => {
    const ids = new Set(graph.nodes.map((node) => node.id));
    setPointerTraceNodeId((current) => current && ids.has(current) ? current : undefined);
    setFocusedTraceNodeId((current) => current && ids.has(current) ? current : undefined);
  }, [graph.nodes]);

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
        elements: graphElements(graph, linkPresentations, selectedNodeRef.current),
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
      setRendererError(errorMessage(error, labels().desktop.topology.graph.init_failed));
      return;
    }

    let labelFrame = 0;
    let connectorFrame = 0;
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
          // Frame headers straddle the frame's top border on a paper plate,
          // the way a map legend names a region. The frame's padding is
          // uniform, so a header inside it would sit on the first row.
          const box = node.renderedBoundingBox({ includeLabels: false, includeOverlays: false });
          label.style.transform = `translate3d(${box.x1 + 14 * zoom}px, ${box.y1}px, 0) scale(${labelScale}) translate(0, -50%)`;
          // A lane header spans its frame so the action sits at the far edge
          // instead of crowding the name.
          if (label.classList.contains("graph-lane-label")) {
            label.style.width = `${Math.max(0, (box.w - 28 * zoom) / labelScale)}px`;
          }
        } else {
          const point = node.renderedPosition();
          label.style.transform = `translate3d(${point.x}px, ${point.y}px, 0) scale(${labelScale}) translate(-50%, -50%)`;
        }
      }
      const traceNodeId = activeTraceNodeRef.current;
      const traceNode = traceNodeId ? cy.getElementById(traceNodeId) : cy.collection();
      const nodeObstacles = cy.nodes()
        .filter((node) => ["resource", "resource-group", "aggregate", "external", "subscription"]
          .includes(node.data("kind")))
        .map((node) => ({
          id: node.id(),
          rect: node.renderedBoundingBox({ includeLabels: false, includeOverlays: false }),
        }));
      const placedLabelRects: LabelRect[] = [];
      for (const [, label] of relationshipLabelRefs.current) {
        const endpointId = label.dataset.endpointId;
        if (!endpointId || traceNode.empty()) continue;
        const endpoint = cy.getElementById(endpointId);
        if (endpoint.empty()) continue;
        const tracePoint = traceNode.renderedPosition();
        const box = endpoint.renderedBoundingBox({ includeLabels: false, includeOverlays: false });
        const slot = Number(label.dataset.endpointSlot ?? "0");
        const placement = placeRelationshipLabel(
          tracePoint,
          box,
          { width: label.offsetWidth, height: label.offsetHeight },
          { x1: 8, y1: 8, x2: activeHost.clientWidth - 8, y2: activeHost.clientHeight - 8 },
          [
            ...nodeObstacles.filter((obstacle) => obstacle.id !== endpointId).map((obstacle) => obstacle.rect),
            ...placedLabelRects,
          ],
          slot,
        );
        label.dataset.side = placement.side;
        label.style.transform = `translate3d(${placement.x}px, ${placement.y}px, 0)`;
        placedLabelRects.push({
          x1: placement.x,
          y1: placement.y,
          x2: placement.x + label.offsetWidth,
          y2: placement.y + label.offsetHeight,
        });
      }
      for (const region of graphRegions) {
        const label = regionLabelRefs.current.get(region.id);
        if (!label) continue;
        const nodes = collectionFor(region.nodeIds);
        if (nodes.empty()) continue;
        const box = nodes.renderedBoundingBox({ includeLabels: false, includeOverlays: false });
        label.style.transform = `translate3d(${box.x1}px, ${Math.max(12, box.y1 - 28)}px, 0)`;
      }
    }

    function queueLabelSync() {
      if (labelFrame !== 0) return;
      labelFrame = window.requestAnimationFrame(syncLabels);
    }

    function shouldAnimate() {
      return visible
        && !reduceMotion.matches
        && motionRef.current
        && cy.edges(".relationship-edge.trace-active").nonempty();
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
      cy.edges(".relationship-edge.trace-active").style("line-dash-offset", dashOffset);
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
      if (["lane", "subnet", "connector-port"].includes(event.target.data("kind"))) return;
      event.target.addClass("hovered");
      setPointerTraceNodeId(event.target.id());
      activeHost.style.cursor = "pointer";
    }

    function handleNodeOut(event: cytoscape.EventObject) {
      event.target.removeClass("hovered");
      setPointerTraceNodeId((current) => current === event.target.id() ? undefined : current);
      activeHost.style.cursor = "grab";
    }

    function applyTrace(nodeId?: string) {
      const node = nodeId ? cy.getElementById(nodeId) : cy.collection();
      cy.batch(() => {
        cy.nodes().removeClass("trace-source trace-peer trace-muted");
        cy.edges(".relationship-edge").removeClass("trace-active trace-muted");
        if (node.empty()) return;
        const activeEdges = cy.edges(".relationship-edge").filter((edge) => (
          edge.data("logicalSource") === nodeId || edge.data("logicalTarget") === nodeId
        ));
        const peerIds = activeEdges.map((edge) => (
          edge.data("logicalSource") === nodeId
            ? edge.data("logicalTarget") as string
            : edge.data("logicalSource") as string
        ));
        const peers = collectionFor(peerIds);
        node.addClass("trace-source");
        peers.addClass("trace-peer");
        cy.nodes()
          .filter(".resource-node, .resource-group-node, .aggregate-node, .external-node, .lane-bar-node, .vnet-frame")
          .difference(node.union(peers))
          .addClass("trace-muted");
        activeEdges.addClass("trace-active");
        cy.edges(".relationship-edge").difference(activeEdges).addClass("trace-muted");
      });
      queueLabelSync();
      scheduleFlow();
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
    cy.on("position", "node:not(.connector-port)", queueConnectorGeometry);
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
      const targetIds = cameraTargetIds(
        plan,
        cy.nodes().not(".connector-port").map((node) => node.id()),
        mode,
        selectedId,
      );
      const requested = collectionFor(targetIds);
      const targetIdSet = new Set(targetIds);
      const connectingEdges = cy.edges(".relationship-edge").filter((edge) => (
        targetIdSet.has(edge.data("logicalSource")) && targetIdSet.has(edge.data("logicalTarget"))
      ));
      const target = requested.nonempty() ? requested.union(connectingEdges) : cy.elements();
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

    function syncConnectorGeometry(plan: TopologyLayoutPlan) {
      const connectorPositions = new Map(plan.positions);
      const connectorBounds = new Map<string, LabelRect>();
      for (const node of cy.nodes().not(".connector-port")) {
        connectorPositions.set(node.id(), node.position());
        connectorBounds.set(
          node.id(),
          node.boundingBox({ includeLabels: false, includeOverlays: false }),
        );
      }
      cy.batch(() => {
        for (const assignment of connectorPortAssignments(
          linkPresentations,
          connectorPositions,
          connectorBounds,
        )) {
          const source = cy.getElementById(assignment.sourceId);
          const target = cy.getElementById(assignment.targetId);
          const sourcePort = cy.getElementById(assignment.sourcePortId);
          const targetPort = cy.getElementById(assignment.targetPortId);
          if (source.empty() || target.empty() || sourcePort.empty() || targetPort.empty()) continue;
          sourcePort.position(connectorPortPosition(
            connectorBounds.get(assignment.sourceId) ?? source.boundingBox({ includeLabels: false, includeOverlays: false }),
            assignment.sourcePort,
          ));
          targetPort.position(connectorPortPosition(
            connectorBounds.get(assignment.targetId) ?? target.boundingBox({ includeLabels: false, includeOverlays: false }),
            assignment.targetPort,
          ));
          cy.getElementById(assignment.edgeId)
            .data("taxiTurn", assignment.taxiTurn)
            .style({
              "taxi-direction": assignment.taxiDirection,
            });
        }
      });
    }

    function queueConnectorGeometry() {
      if (connectorFrame !== 0) return;
      connectorFrame = window.requestAnimationFrame(() => {
        connectorFrame = 0;
        const plan = layoutPlanRef.current;
        if (plan) syncConnectorGeometry(plan);
      });
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
      syncConnectorGeometry(plan);
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
      setRendererError(errorMessage(error, labels().desktop.topology.graph.layout_failed));
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
    traceApiRef.current = applyTrace;
    labelSyncRef.current = queueLabelSync;
    motionSchedulerRef.current = scheduleFlow;
    applyTrace(activeTraceNodeRef.current);
    scheduleFlow();
    queueLabelSync();

    return () => {
      window.cancelAnimationFrame(labelFrame);
      window.cancelAnimationFrame(connectorFrame);
      window.cancelAnimationFrame(resizeFrame);
      window.cancelAnimationFrame(motionFrame);
      resizeObserver.disconnect();
      surface.removeEventListener("pointerdown", markUserAdjusted);
      surface.removeEventListener("wheel", markUserAdjusted);
      document.removeEventListener("visibilitychange", handleVisibility);
      reduceMotion.removeEventListener("change", handleMotionPreference);
      cameraApiRef.current = undefined;
      activationApiRef.current = undefined;
      traceApiRef.current = undefined;
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

  function typeIcon(azureType?: string | null) {
    const type = azureType ? typeMap.get(azureType) : undefined;
    return type?.icon;
  }

  function typeName(azureType?: string | null) {
    const type = azureType ? typeMap.get(azureType) : undefined;
    return type?.displayName ?? azureType ?? words.resource_fallback;
  }

  function findingText(node: TopologyNode) {
    return node.findingCount > 0 ? plural(words.findings_suffix, node.findingCount) : "";
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

  function graphButtonProps(id: string, traceable = true) {
    return {
      tabIndex: keyboardNodeId === id ? 0 : -1,
      onFocus: () => {
        setKeyboardNodeId(id);
        if (traceable) setFocusedTraceNodeId(id);
        cyRef.current?.getElementById(id).addClass("keyboard-focus");
      },
      onBlur: () => {
        if (traceable) setFocusedTraceNodeId((current) => current === id ? undefined : current);
        cyRef.current?.getElementById(id).removeClass("keyboard-focus");
      },
      onPointerEnter: () => {
        if (traceable) setPointerTraceNodeId(id);
      },
      onPointerLeave: () => {
        if (traceable) setPointerTraceNodeId((current) => current === id ? undefined : current);
      },
      onKeyDown: (event: ReactKeyboardEvent<HTMLButtonElement>) => handleGraphButtonKeyDown(id, event),
      "data-graph-roving": true,
      "data-graph-node-id": id,
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

  const registerRelationshipLabel = (id: string) => (element: HTMLElement | null) => {
    if (element) relationshipLabelRefs.current.set(id, element);
    else relationshipLabelRefs.current.delete(id);
  };

  const registerRegionLabel = (id: string) => (element: HTMLElement | null) => {
    if (element) regionLabelRefs.current.set(id, element);
    else regionLabelRefs.current.delete(id);
  };

  function traceClass(id: string) {
    if (!activeTraceNodeId) return "";
    if (id === activeTraceNodeId) return "trace-source";
    return activePeerIds.has(id) ? "trace-peer" : "trace-muted";
  }

  function tracedClassName(base: string, id: string, ...conditional: Array<string | false | undefined>) {
    return [base, traceClass(id), ...conditional].filter(Boolean).join(" ");
  }

  function kindMarks(id: string) {
    const meta = connectionMeta.get(id);
    if (!meta || meta.kinds.size === 0) return null;
    return (
      <span className="graph-kind-marks" aria-hidden="true">
        {[...meta.kinds].sort().map((kindClass) => (
          <i key={kindClass} style={{ background: kindClassColor(kindClass) }} />
        ))}
      </span>
    );
  }

  function representativeTypes(node: TopologyNode) {
    const counts = new Map<string, number>();
    for (const id of node.memberIds) {
      const azureType = resourceMap.get(id)?.azureType;
      if (azureType) counts.set(azureType, (counts.get(azureType) ?? 0) + 1);
    }
    return [...counts.entries()]
      .sort((left, right) => right[1] - left[1] || (left[0] < right[0] ? -1 : left[0] > right[0] ? 1 : 0))
      .slice(0, 3)
      .map(([azureType]) => ({ azureType, icon: typeIcon(azureType) }))
      .filter((entry): entry is { azureType: string; icon: string } => Boolean(entry.icon));
  }

  return (
    <div className="cytoscape-graph" ref={hostRef}>
      <div className="graph-aurora" aria-hidden="true" />
      <div className="cytoscape-surface" ref={surfaceRef} aria-hidden="true" />
      <div className="graph-region-label-layer" aria-hidden="true">
        {graphRegions.map((region) => (
          <span key={region.id} ref={registerRegionLabel(region.id)}>
            <strong>{region.label}</strong>
            <small>{region.count}</small>
          </span>
        ))}
      </div>
      <div className="graph-relationship-label-layer" aria-hidden="true">
        {activeLabels.map((label: ActiveRelationshipLabel) => (
          <span
            key={label.edgeId}
            ref={registerRelationshipLabel(label.edgeId)}
            className="graph-relationship-label"
            data-endpoint-id={label.endpointId}
            data-endpoint-slot={label.endpointSlot}
          >
            <i style={{ background: kindClassColor(label.kindClass) }} />
            {label.label}
          </span>
        ))}
      </div>
      <div className="graph-label-layer">
        {graph.lanes
          .filter((lane) => lane.expanded && graph.nodes.some((node) => laneMember(node) && node.lane === lane.subscriptionId))
          .map((lane) => (
            <button
              key={`lane:${lane.subscriptionId}`}
              ref={registerLabel(`lane:${lane.subscriptionId}`)}
              className="graph-container-label graph-lane-label"
              onClick={() => onActivate({ kind: "subscription", subscriptionId: lane.subscriptionId })}
              aria-label={fill(words.lane_expanded, { name: lane.name, groups: lane.groupCount })}
              {...graphButtonProps(`lane:${lane.subscriptionId}`, false)}
            >
              <img src={SUBSCRIPTION_ICON} alt="" />
              <strong>{lane.name}</strong>
              <small>
                {fill(extra.lane_summary, { groups: plural(extra.group_count, lane.groupCount), resources: lane.resourceCount })}
              </small>
              <i>{extra.collapse_lane}</i>
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
                className={tracedClassName("graph-container-label graph-vnet-label", node.id)}
                onClick={() => requestActivation(
                  { kind: "resource", resourceId: node.resourceId ?? node.id },
                  node.id,
                )}
                aria-pressed={node.id === selectedNodeId}
                aria-label={fill(words.vnet_card, { name: node.name, findings: findingText(node) })}
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
                className={tracedClassName("graph-lane-bar-label", node.id)}
                onClick={() => onActivate({ kind: "subscription", subscriptionId: node.lane ?? "" })}
                aria-label={fill(words.subscription_collapsed, { name: node.name, groups: node.count, findings: findingText(node) })}
                {...graphButtonProps(node.id)}
              >
                <img src={SUBSCRIPTION_ICON} alt="" />
                <span className="graph-node-copy">
                  <strong>{node.name}</strong>
                  <small>{node.subtitle}</small>
                </span>
                {node.findingCount > 0 ? <em aria-label={fill(words.findings_badge, { count: node.findingCount })}>{node.findingCount}</em> : null}
                {kindMarks(node.id)}
                <span className="graph-route-count"><GitBranch size={12} />{connectionMeta.get(node.id)?.connectors ?? 0}</span>
                <i className="graph-lane-expand">{words.expand}</i>
              </button>
            );
          }
          if (node.kind === "aggregate" && node.groupIds.length > 0) {
            // An estate-level tile: the groups nothing crosses into, one per
            // subscription. Expanding lists the groups; each opens its map.
            const expanded = node.id === expandedAggregateId;
            const groups = node.groupIds
              .map((id) => estate.resourceGroupSummaries.find((group) => group.id === id))
              .filter((group): group is NonNullable<typeof group> => Boolean(group))
              .sort((left, right) => (left.name < right.name ? -1 : left.name > right.name ? 1 : 0));
            return (
              <div
                key={node.id}
                ref={registerLabel(node.id)}
                className={tracedClassName(
                  "graph-aggregate-cluster groups",
                  node.id,
                  expanded && "expanded",
                )}
                onKeyDown={(event) => {
                  if (event.key !== "Escape" || !expanded) return;
                  event.preventDefault();
                  onActivate({ kind: "aggregate", nodeId: node.id });
                  event.currentTarget.querySelector<HTMLButtonElement>("[data-graph-roving]")?.focus();
                }}
              >
                <button
                  className={expanded ? "graph-aggregate-label groups selected" : "graph-aggregate-label groups"}
                  onClick={() => onActivate({ kind: "aggregate", nodeId: node.id })}
                  aria-expanded={expanded}
                  aria-label={fill(words.aggregate_groups, { count: node.count, findings: findingText(node), action: expanded ? words.collapse : words.show })}
                  {...graphButtonProps(node.id)}
                >
                  <img src={RESOURCE_GROUP_ICON} alt="" />
                  <span className="graph-node-copy">
                    <strong>{node.name}</strong>
                    <small>{words.no_cross_group}</small>
                  </span>
                  <b>{node.subtitle}</b>
                  {node.findingCount > 0 ? <em aria-label={fill(words.findings_badge, { count: node.findingCount })}>{node.findingCount}</em> : null}
                </button>
                {expanded ? (
                  <div className="graph-aggregate-members" role="group" aria-label={words.unconnected_groups_aria}>
                    {groups.map((group) => (
                      <button key={group.id} onClick={() => requestActivation(
                        { kind: "resource-group", groupId: group.id },
                        node.id,
                      )} title={group.name}>
                        <img src={RESOURCE_GROUP_ICON} alt="" />
                        <span><strong>{group.name}</strong><small>{fill(extra.member_group, { resources: plural(labels().common.plurals.resource, group.resourceCount, { count: group.resourceCount }) })}</small></span>
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
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
                className={tracedClassName(
                  "graph-aggregate-cluster",
                  node.id,
                  expanded && "expanded",
                )}
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
                  aria-label={fill(words.aggregate_resources, { count: node.count, type: typeName(node.azureType), findings: findingText(node), action: expanded ? words.collapse : words.show })}
                  {...graphButtonProps(node.id)}
                >
                  {typeIcon(node.azureType) ? <img src={typeIcon(node.azureType)} alt="" /> : null}
                  <span className="graph-node-copy">
                    <strong>{typeName(node.azureType)}</strong>
                    <small>{node.zone === "unconnected" ? words.no_drawn_connectors : plural(words.hops_away, node.hop ?? 1)}</small>
                  </span>
                  <b>{node.subtitle}</b>
                  {node.findingCount > 0 ? <em aria-label={fill(words.findings_badge, { count: node.findingCount })}>{node.findingCount}</em> : null}
                </button>
                {expanded ? (
                  <div className="graph-aggregate-members" role="group" aria-label={fill(extra.members_aria, { type: typeName(node.azureType) })}>
                    {members.map((resource) => (
                      <button key={resource.id} onClick={() => requestActivation(
                        { kind: "resource", resourceId: resource.id },
                        node.id,
                      )} title={resource.name}>
                        {typeIcon(resource.azureType) ? <img src={typeIcon(resource.azureType)} alt="" /> : null}
                        <span><strong>{resource.name}</strong><small>{resource.findingCount > 0 ? fill(words.findings_badge, { count: resource.findingCount }) : words.open_record}</small></span>
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
                className={tracedClassName(
                  "graph-node-label ghost",
                  node.id,
                  node.id === selectedNodeId && "selected",
                )}
                onClick={() => node.resourceId
                  ? requestActivation({ kind: "resource", resourceId: node.resourceId }, node.id)
                  : onActivate({ kind: "aggregate", nodeId: node.id })}
                aria-pressed={node.id === selectedNodeId}
                aria-label={fill(words.external_card, { name: node.name, findings: findingText(node) })}
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
            const meta = connectionMeta.get(node.id);
            const icons = representativeTypes(node);
            return (
              <button
                key={node.id}
                ref={registerLabel(node.id)}
                className={tracedClassName(
                  "graph-resource-group-label",
                  node.id,
                  selected && "selected",
                )}
                onClick={() => requestActivation(
                  { kind: "resource-group", groupId: node.groupId ?? "" },
                  node.id,
                )}
                aria-pressed={selected}
                aria-label={fill(words.group_card, { name: node.name, count: node.count, findings: findingText(node) })}
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
                  {node.findingCount > 0 ? <em aria-label={fill(words.findings_badge, { count: node.findingCount })}>{node.findingCount}</em> : null}
                </span>
                <span className="graph-group-types" aria-hidden="true">
                  <span className="graph-group-type-icons">
                    {icons.map((entry) => <img key={entry.azureType} src={entry.icon} alt="" />)}
                  </span>
                  {kindMarks(node.id)}
                  <span className="graph-route-count"><GitBranch size={12} />{meta?.connectors ?? 0}</span>
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
                className={tracedClassName(
                  "graph-node-label graph-node-actions",
                  node.id,
                  selected && "selected",
                  dim && "dimmed",
                )}
                title={node.name}
                onPointerEnter={() => setPointerTraceNodeId(node.id)}
                onPointerLeave={() => setPointerTraceNodeId((current) => current === node.id ? undefined : current)}
              >
                <button
                  className="graph-node-primary"
                  onClick={() => requestActivation({ kind: "resource", resourceId }, node.id)}
                  aria-pressed={selected}
                  aria-label={fill(words.resource_card, {
                    name: node.name,
                    type: typeName(node.azureType),
                    findings: findingText(node),
                    relationships: relationshipCount > 0
                      ? fill(words.explore_hint, { count: relationshipCount })
                      : words.no_relationships_hint,
                  })}
                  {...resourceButtonProps(node.id, resourceId, relationshipCount)}
                >
                  <span className="graph-node-icon">
                    {typeIcon(node.azureType) ? <img src={typeIcon(node.azureType)} alt="" /> : null}
                  </span>
                  <span className="graph-node-copy">
                    <strong>{node.name}</strong>
                    <small>{node.subtitle || typeName(node.azureType)}</small>
                  </span>
                  {node.findingCount > 0 ? <em aria-label={fill(words.findings_badge, { count: node.findingCount })}>{node.findingCount}</em> : null}
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
                    ? fill(words.explore_aria, { count: relationshipCount, name: node.name })
                    : fill(words.no_relationships_aria, { name: node.name })}
                  title={relationshipCount > 0
                    ? fill(words.explore, { count: relationshipCount })
                    : words.no_relationships}
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
        <EmptyState
          className="graph-empty-state"
          role="status"
          icon={<img src={RESOURCE_GROUP_ICON} alt="" />}
          title={words.nothing_title}
          detail={words.nothing_detail}
        />
      ) : null}
      {rendererError ? (
        <div className="graph-renderer-error" role="alert" aria-live="assertive">
          <strong>{extra.renderer_unavailable}</strong>
          <span>{rendererError}</span>
          <p>{extra.renderer_retry_detail}</p>
          <button onClick={() => setRendererRetryNonce((value) => value + 1)}>{extra.renderer_retry}</button>
        </div>
      ) : null}
    </div>
  );
}
