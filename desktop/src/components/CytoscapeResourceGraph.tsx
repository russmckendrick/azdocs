import { useEffect, useMemo, useRef, useState } from "react";
import cytoscape, { type Core, type ElementDefinition, type StylesheetJson } from "cytoscape";
import type { Edge, EstateSnapshot, Resource } from "../types";
import {
  RESOURCE_GROUP_ICON,
  GRAPH_NODE_LIMITS,
  buildResourceGroupTopology,
  edgesWithinResources,
  resourceGroupNodeId,
  resourcesInGroup,
  type ResourceGroupSummary,
} from "./topology-model";

export type GraphMode = "neighbourhood" | "estate";

type GraphLevel = "resource-groups" | "resources";

interface GraphNode {
  id: string;
  name: string;
  kind: "resource" | "resource-group";
  resource?: Resource;
  resourceGroup?: ResourceGroupSummary;
}

interface GraphLink {
  sourceId: string;
  targetId: string;
  label: string;
  count: number;
}

interface GraphData {
  level: GraphLevel;
  nodes: GraphNode[];
  links: GraphLink[];
  roots: string[];
}

interface CytoscapeResourceGraphProps {
  estate: EstateSnapshot;
  selectedResourceId: string;
  selectedResourceGroupId?: string;
  resourceGroupId?: string;
  mode: GraphMode;
  motionEnabled: boolean;
  focusNonce: number;
  onSelectResource: (id: string) => void;
  onSelectResourceGroup: (id: string) => void;
  onOpenResourceGroup: (id: string) => void;
}

const NODE_WIDTH = 172;
const NODE_HEIGHT = 76;

function stableCompare(left: string, right: string) {
  return left < right ? -1 : left > right ? 1 : 0;
}

function sortedResources(resources: Resource[]) {
  return [...resources].sort((left, right) =>
    stableCompare(left.name, right.name) || stableCompare(left.id, right.id),
  );
}

function sortedEdges(edges: Edge[]) {
  return [...edges].sort((left, right) =>
    stableCompare(left.kind, right.kind)
      || stableCompare(left.sourceId, right.sourceId)
      || stableCompare(left.targetId, right.targetId),
  );
}

function directedRoots(nodes: GraphNode[], links: GraphLink[]) {
  const nodeMap = new Map(nodes.map((node) => [node.id, node]));
  const adjacency = new Map(nodes.map((node) => [node.id, [] as string[]]));
  const incoming = new Map(nodes.map((node) => [node.id, 0]));
  for (const link of links) {
    if (!nodeMap.has(link.sourceId) || !nodeMap.has(link.targetId)) continue;
    adjacency.get(link.sourceId)?.push(link.targetId);
    adjacency.get(link.targetId)?.push(link.sourceId);
    incoming.set(link.targetId, (incoming.get(link.targetId) ?? 0) + 1);
  }
  adjacency.forEach((neighbors) => neighbors.sort());

  const remaining = new Set(nodes.map((node) => node.id));
  const roots: string[] = [];
  while (remaining.size > 0) {
    const first = [...remaining].sort()[0];
    const queue = [first];
    const component: string[] = [];
    remaining.delete(first);
    while (queue.length > 0) {
      const current = queue.shift();
      if (!current) continue;
      component.push(current);
      for (const neighbor of adjacency.get(current) ?? []) {
        if (!remaining.delete(neighbor)) continue;
        queue.push(neighbor);
      }
    }
    const sourceRoots = component.filter((id) => (incoming.get(id) ?? 0) === 0);
    const candidates = sourceRoots.length > 0 ? sourceRoots : component;
    candidates.sort((left, right) => {
      const degreeDifference = (adjacency.get(right)?.length ?? 0) - (adjacency.get(left)?.length ?? 0);
      return degreeDifference
        || stableCompare(nodeMap.get(left)?.name ?? left, nodeMap.get(right)?.name ?? right)
        || stableCompare(left, right);
    });
    roots.push(...candidates);
  }
  return roots;
}

function orderGroupNodes(nodes: GraphNode[], links: GraphLink[]) {
  const incoming = new Map(nodes.map((node) => [node.id, 0]));
  const outgoing = new Map(nodes.map((node) => [node.id, 0]));
  const degree = new Map(nodes.map((node) => [node.id, 0]));
  for (const link of links) {
    incoming.set(link.targetId, (incoming.get(link.targetId) ?? 0) + 1);
    outgoing.set(link.sourceId, (outgoing.get(link.sourceId) ?? 0) + 1);
    degree.set(link.sourceId, (degree.get(link.sourceId) ?? 0) + 1);
    degree.set(link.targetId, (degree.get(link.targetId) ?? 0) + 1);
  }
  return [...nodes].sort((left, right) =>
    Number((degree.get(left.id) ?? 0) === 0) - Number((degree.get(right.id) ?? 0) === 0)
      || (incoming.get(left.id) ?? 0) - (incoming.get(right.id) ?? 0)
      || (outgoing.get(right.id) ?? 0) - (outgoing.get(left.id) ?? 0)
      || stableCompare(left.name, right.name)
      || stableCompare(left.id, right.id),
  );
}

function resourceGraphNodes(resources: Resource[]): GraphNode[] {
  return resources.map((resource) => ({
    id: resource.id,
    name: resource.name,
    kind: "resource",
    resource,
  }));
}

function resourceGraphLinks(edges: Edge[]): GraphLink[] {
  return sortedEdges(edges).map((edge) => ({
    sourceId: edge.sourceId,
    targetId: edge.targetId,
    label: edge.kind.replaceAll("_", " "),
    count: 1,
  }));
}

function buildGraphData(
  estate: EstateSnapshot,
  selectedResourceId: string,
  mode: GraphMode,
  resourceGroupId?: string,
): GraphData {
  const resources = sortedResources(estate.resources);
  const selected = resources.find((resource) => resource.id === selectedResourceId) ?? resources[0];
  if (!selected) return { level: "resources", nodes: [], links: [], roots: [] };

  if (mode === "neighbourhood") {
    const touchingEdges = sortedEdges(estate.edges.filter(
      (edge) => edge.sourceId === selected.id || edge.targetId === selected.id,
    ));
    const neighborIds = new Set(touchingEdges.flatMap((edge) => [edge.sourceId, edge.targetId]));
    const neighbors = resources
      .filter((resource) => resource.id !== selected.id && neighborIds.has(resource.id))
      .slice(0, GRAPH_NODE_LIMITS.neighbourhood - 1);
    const visibleResources = [selected, ...neighbors];
    const visibleIds = new Set(visibleResources.map((resource) => resource.id));
    const nodes = resourceGraphNodes(visibleResources);
    const links = resourceGraphLinks(
      touchingEdges.filter((edge) => visibleIds.has(edge.sourceId) && visibleIds.has(edge.targetId)),
    );
    return {
      level: "resources",
      nodes,
      links,
      roots: [selected.id],
    };
  }

  const topology = buildResourceGroupTopology(estate);
  if (!resourceGroupId) {
    const unorderedNodes: GraphNode[] = topology.groups.map((group) => ({
      id: resourceGroupNodeId(group.id),
      name: group.name,
      kind: "resource-group",
      resourceGroup: group,
    }));
    const links: GraphLink[] = topology.links.map((link) => ({
      sourceId: resourceGroupNodeId(link.sourceId),
      targetId: resourceGroupNodeId(link.targetId),
      label: `${link.count} cross-group link${link.count === 1 ? "" : "s"}`,
      count: link.count,
    }));
    const nodes = orderGroupNodes(unorderedNodes, links);
    return {
      level: "resource-groups",
      nodes,
      links,
      roots: directedRoots(nodes, links),
    };
  }

  const group = topology.groups.find((candidate) => candidate.id === resourceGroupId);
  const groupResources = resourcesInGroup(estate, group);
  const preferred = groupResources
    .filter((resource) => resource.id !== selectedResourceId)
    .sort((left, right) =>
      right.edgeCount - left.edgeCount
        || right.findingCount - left.findingCount
        || stableCompare(left.name, right.name)
        || stableCompare(left.id, right.id),
    );
  const selectedInGroup = groupResources.find((resource) => resource.id === selectedResourceId);
  const visibleResources = [
    ...(selectedInGroup ? [selectedInGroup] : []),
    ...preferred,
  ].slice(0, GRAPH_NODE_LIMITS.resourceGroup)
    .sort((left, right) => stableCompare(left.name, right.name) || stableCompare(left.id, right.id));
  const visibleIds = new Set(visibleResources.map((resource) => resource.id));
  const visibleEdges = edgesWithinResources(estate.edges, groupResources).filter(
    (edge) => visibleIds.has(edge.sourceId) && visibleIds.has(edge.targetId),
  );
  const nodes = resourceGraphNodes(visibleResources);
  const links = resourceGraphLinks(visibleEdges);
  return {
    level: "resources",
    nodes,
    links,
    roots: directedRoots(nodes, links),
  };
}

function graphStyles(): StylesheetJson {
  return [
    {
      selector: "node.resource-node",
      style: {
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        shape: "round-rectangle",
        "corner-radius": "12px",
        "background-color": "#102338",
        "background-opacity": 0.62,
        "border-width": 1,
        "border-color": "#365673",
        "border-opacity": 0.82,
        "overlay-opacity": 0,
        "underlay-opacity": 0,
        label: "",
        "z-index": 8,
      },
    },
    {
      selector: "node.resource-group-node",
      style: {
        width: 244,
        height: 118,
        shape: "round-rectangle",
        "corner-radius": "15px",
        "background-color": "#10283e",
        "background-opacity": 0.76,
        "border-width": 1,
        "border-color": "#3b7397",
        "border-opacity": 0.78,
        "overlay-opacity": 0,
        "underlay-opacity": 0,
        label: "",
        "z-index": 8,
      },
    },
    {
      selector: "node.focused",
      style: {
        "background-color": "#0a5685",
        "background-opacity": 0.72,
        "border-width": 1.5,
        "border-color": "#72ddff",
        "border-opacity": 1,
        "z-index": 14,
      },
    },
    {
      selector: "node.hovered",
      style: {
        "background-opacity": 0.78,
        "border-width": 1.5,
        "border-color": "#67c9ec",
      },
    },
    {
      selector: "node:grabbed",
      style: {
        "background-opacity": 0.86,
        "border-width": 2,
        "border-color": "#8ce7ff",
      },
    },
    {
      selector: "node.keyboard-focus",
      style: {
        "background-opacity": 0.86,
        "border-width": 2,
        "border-color": "#8ce7ff",
      },
    },
    {
      selector: "edge.relationship-edge",
      style: {
        width: 1.35,
        "curve-style": "round-taxi",
        "taxi-direction": "horizontal",
        "taxi-turn": "50%",
        "taxi-turn-min-distance": "28px",
        "taxi-radius": 9,
        "edge-distances": "intersection",
        "line-color": "#4a9cc5",
        "line-opacity": 0.72,
        "line-cap": "round",
        "target-arrow-shape": "triangle",
        "target-arrow-color": "#77dfff",
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
        width: 1.8,
        "line-color": "#63c9ed",
        "line-opacity": 0.9,
        "target-arrow-color": "#9cecff",
      },
    },
    {
      selector: "edge.relationship-edge.labelled",
      style: {
        label: "data(label)",
        color: "#a6d8ef",
        "font-family": "Plex Mono, monospace",
        "font-size": 7,
        "font-weight": 600,
        "text-transform": "uppercase",
        "text-background-color": "#071522",
        "text-background-opacity": 0.94,
        "text-background-padding": "5px",
        "text-background-shape": "roundrectangle",
        "text-border-color": "#39749a",
        "text-border-opacity": 0.45,
        "text-border-width": 1,
      },
    },
    {
      selector: "edge.group-link",
      style: {
        width: "mapData(weight, 1, 20, 1.3, 3.4)",
        "line-color": "#3f91bd",
        "line-opacity": 0.68,
        "target-arrow-color": "#73d8fa",
        "arrow-scale": 0.68,
        color: "#8fc8e6",
        "font-size": 8,
        "text-background-padding": "6px",
        "text-background-opacity": 0.9,
      },
    },
    {
      selector: "edge.flow-edge",
      style: {
        width: 1.15,
        "curve-style": "round-taxi",
        "taxi-direction": "horizontal",
        "taxi-turn": "50%",
        "taxi-turn-min-distance": "28px",
        "taxi-radius": 9,
        "edge-distances": "intersection",
        "line-style": "dashed",
        "line-dash-pattern": [2, 14],
        "line-color": "#9beaff",
        "line-opacity": 0.76,
        "source-distance-from-node": "5px",
        "target-distance-from-node": "7px",
        "target-arrow-shape": "none",
        "overlay-opacity": 0,
        events: "no",
        "z-index": 3,
      },
    },
  ];
}

function graphElements(graph: GraphData, focusedNodeId: string, mode: GraphMode): ElementDefinition[] {
  const elements: ElementDefinition[] = graph.nodes.map((node) => ({
    group: "nodes",
    data: {
      id: node.id,
      kind: node.kind,
      resourceId: node.resource?.id,
      resourceGroupId: node.resourceGroup?.id,
    },
    classes: [
      node.kind === "resource-group" ? "resource-group-node" : "resource-node",
      node.id === focusedNodeId ? "focused" : "",
    ].filter(Boolean).join(" "),
  }));
  graph.links.forEach((link, index) => {
    const related = link.sourceId === focusedNodeId || link.targetId === focusedNodeId;
    const groupLink = graph.level === "resource-groups";
    const edgeClass = [
      "relationship-edge",
      related ? "related" : "",
      mode === "neighbourhood" || groupLink ? "labelled" : "",
      groupLink ? "group-link" : "",
    ]
      .filter(Boolean)
      .join(" ");
    elements.push({
      group: "edges",
      data: {
        id: `relationship-${index}`,
        source: link.sourceId,
        target: link.targetId,
        label: link.label,
        weight: link.count,
      },
      classes: edgeClass,
    });
    elements.push({
      group: "edges",
      data: { id: `flow-${index}`, source: link.sourceId, target: link.targetId },
      classes: "flow-edge",
    });
  });
  return elements;
}

export function CytoscapeResourceGraph({
  estate,
  selectedResourceId,
  selectedResourceGroupId,
  resourceGroupId,
  mode,
  motionEnabled,
  focusNonce,
  onSelectResource,
  onSelectResourceGroup,
  onOpenResourceGroup,
}: CytoscapeResourceGraphProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const surfaceRef = useRef<HTMLDivElement>(null);
  const labelRefs = useRef(new Map<string, HTMLButtonElement>());
  const cyRef = useRef<Core | undefined>(undefined);
  const selectRef = useRef(onSelectResource);
  const selectGroupRef = useRef(onSelectResourceGroup);
  const openGroupRef = useRef(onOpenResourceGroup);
  const motionRef = useRef(motionEnabled);
  const [rendererError, setRendererError] = useState<string>();
  const graph = useMemo(
    () => buildGraphData(estate, selectedResourceId, mode, resourceGroupId),
    [estate, mode, resourceGroupId, selectedResourceId],
  );
  const focusedNodeId = graph.level === "resource-groups" && selectedResourceGroupId
    ? resourceGroupNodeId(selectedResourceGroupId)
    : selectedResourceId;
  const graphStructureKey = useMemo(() => JSON.stringify({
    mode,
    level: graph.level,
    nodes: graph.nodes.map((node) => node.id),
    links: graph.links.map((link) => [link.sourceId, link.targetId, link.label, link.count]),
  }), [graph.level, graph.links, graph.nodes, mode]);
  const typeMap = useMemo(
    () => new Map(estate.resourceTypes.map((type) => [type.azureType, type])),
    [estate.resourceTypes],
  );

  useEffect(() => {
    selectRef.current = onSelectResource;
  }, [onSelectResource]);

  useEffect(() => {
    selectGroupRef.current = onSelectResourceGroup;
    openGroupRef.current = onOpenResourceGroup;
  }, [onOpenResourceGroup, onSelectResourceGroup]);

  useEffect(() => {
    motionRef.current = motionEnabled;
    cyRef.current?.edges(".flow-edge").style("display", motionEnabled ? "element" : "none");
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
      return;
    }
    cy.animate(
      { fit: { eles: selected, padding: 150 } },
      { duration: 420, easing: "ease-out-cubic" },
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
        elements: graphElements(graph, focusedNodeId, mode),
        style: graphStyles(),
        layout: { name: "preset" },
        minZoom: 0.34,
        maxZoom: 2.2,
        boxSelectionEnabled: true,
        selectionType: "single",
        pixelRatio: "auto",
      });
      cyRef.current = cy;
      cy.edges(".flow-edge").style("display", motionRef.current ? "element" : "none");
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

    function syncLabels() {
      labelFrame = 0;
      const zoom = cy.zoom();
      for (const graphNode of graph.nodes) {
        const label = labelRefs.current.get(graphNode.id);
        const node = cy.getElementById(graphNode.id);
        if (!label || node.empty()) continue;
        const point = node.renderedPosition();
        label.style.transform = `translate3d(${point.x}px, ${point.y}px, 0) translate(-50%, -50%) scale(${zoom})`;
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
      dashOffset = (dashOffset - 0.7) % 16;
      cy.edges(".flow-edge").style("line-dash-offset", dashOffset);
    }

    function handleVisibility() {
      visible = document.visibilityState === "visible";
    }

    function handleTap(event: cytoscape.EventObject) {
      if (event.target.data("kind") === "resource-group") {
        selectGroupRef.current(event.target.data("resourceGroupId"));
        return;
      }
      selectRef.current(event.target.data("resourceId") ?? event.target.id());
    }

    function handleDoubleTap(event: cytoscape.EventObject) {
      const groupId = event.target.data("resourceGroupId");
      if (groupId) openGroupRef.current(groupId);
    }

    function handleNodeOver(event: cytoscape.EventObject) {
      event.target.addClass("hovered");
      activeHost.style.cursor = "pointer";
    }

    function handleNodeOut(event: cytoscape.EventObject) {
      event.target.removeClass("hovered");
      activeHost.style.cursor = "grab";
    }

    function handleGrab() {
      activeHost.style.cursor = "grabbing";
    }

    function handleFree() {
      activeHost.style.cursor = "pointer";
    }

    cy.on("tap", "node", handleTap);
    cy.on("dbltap", "node.resource-group-node", handleDoubleTap);
    cy.on("mouseover", "node", handleNodeOver);
    cy.on("mouseout", "node", handleNodeOut);
    cy.on("grab", "node", handleGrab);
    cy.on("free", "node", handleFree);
    cy.on("pan zoom position resize", queueLabelSync);
    document.addEventListener("visibilitychange", handleVisibility);

    const nodeMap = new Map(graph.nodes.map((node) => [node.id, node]));
    const compareNodes = (left: cytoscape.NodeSingular | null, right: cytoscape.NodeSingular | null) => {
      if (!left) return right ? 1 : 0;
      if (!right) return -1;
      return stableCompare(
        nodeMap.get(left.id())?.name ?? left.id(),
        nodeMap.get(right.id())?.name ?? right.id(),
      ) || stableCompare(left.id(), right.id());
    };

    try {
      if (mode === "neighbourhood") {
        cy.layout({
          name: "breadthfirst",
          roots: graph.roots,
          directed: false,
          direction: "rightward",
          circle: false,
          grid: true,
          maximal: true,
          avoidOverlap: true,
          nodeDimensionsIncludeLabels: false,
          spacingFactor: 1.35,
          padding: 112,
          fit: true,
          animate: false,
          depthSort: compareNodes,
          stop: queueLabelSync,
        }).run();
      } else {
        const structuralElements = cy.nodes().union(cy.edges(".relationship-edge"));
        const components = structuralElements.components();
        const connectedComponents = components
          .filter((component) => component.nodes().length > 1)
          .sort((left, right) => {
            const countDifference = right.nodes().length - left.nodes().length;
            if (countDifference !== 0) return countDifference;
            const leftKey = left.nodes()
              .map((node) => `${nodeMap.get(node.id())?.name ?? node.id()}\0${node.id()}`)
              .sort(stableCompare)[0] ?? "";
            const rightKey = right.nodes()
              .map((node) => `${nodeMap.get(node.id())?.name ?? node.id()}\0${node.id()}`)
              .sort(stableCompare)[0] ?? "";
            return stableCompare(leftKey, rightKey);
          });
        const isolatedNodes = cy.nodes().filter((node) => node.degree(false) === 0);

        let cursorX = 0;
        let cursorY = 0;
        let rowHeight = 0;
        const shelfWidth = 940;
        const componentGap = 96;
        const rowGap = 88;

        for (const component of connectedComponents) {
          const componentIds = new Set(component.nodes().map((node) => node.id()));
          const componentNodes = graph.nodes.filter((node) => componentIds.has(node.id));
          const componentLinks = graph.links.filter(
            (link) => componentIds.has(link.sourceId) && componentIds.has(link.targetId),
          );
          component.layout({
            name: "breadthfirst",
            roots: directedRoots(componentNodes, componentLinks),
            directed: true,
            direction: "rightward",
            circle: false,
            grid: true,
            maximal: true,
            avoidOverlap: true,
            nodeDimensionsIncludeLabels: false,
            spacingFactor: 1.28,
            fit: false,
            animate: false,
            depthSort: compareNodes,
          }).run();
          const bounds = component.boundingBox();
          if (cursorX > 0 && cursorX + bounds.w > shelfWidth) {
            cursorX = 0;
            cursorY += rowHeight + rowGap;
            rowHeight = 0;
          }
          component.nodes().shift({ x: cursorX - bounds.x1, y: cursorY - bounds.y1 });
          cursorX += bounds.w + componentGap;
          rowHeight = Math.max(rowHeight, bounds.h);
        }

        if (isolatedNodes.length > 0) {
          cursorY += rowHeight + rowGap;
          isolatedNodes.layout({
            name: "grid",
            cols: Math.min(5, Math.ceil(Math.sqrt(isolatedNodes.length * 1.6))),
            condense: true,
            avoidOverlap: true,
            avoidOverlapPadding: 34,
            fit: false,
            animate: false,
            sort: compareNodes,
          }).run();
          const isolateBounds = isolatedNodes.boundingBox();
          isolatedNodes.shift({ x: -isolateBounds.x1, y: cursorY - isolateBounds.y1 });
        }
        cy.fit(cy.elements(), 94);
        queueLabelSync();
      }
      if (focusNonce > 0 && graph.level === "resources") {
        const selected = cy.getElementById(focusedNodeId);
        if (!selected.empty()) cy.fit(selected, 150);
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
        if (graph.level === "resource-groups") cy.fit(cy.elements(), 94);
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
      document.removeEventListener("visibilitychange", handleVisibility);
      cyRef.current = undefined;
      cy.destroy();
    };
  }, [graphStructureKey, mode]);

  return (
    <div className="cytoscape-graph" ref={hostRef}>
      <div className="graph-aurora" aria-hidden="true" />
      <div className="cytoscape-surface" ref={surfaceRef} aria-hidden="true" />
      <div className="graph-label-layer">
        {graph.nodes.map((node) => {
          if (node.kind === "resource-group" && node.resourceGroup) {
            const group = node.resourceGroup;
            const selected = node.id === focusedNodeId;
            return (
              <button
                key={node.id}
                ref={(element) => {
                  if (element) labelRefs.current.set(node.id, element);
                  else labelRefs.current.delete(node.id);
                }}
                className={selected ? "graph-resource-group-label selected" : "graph-resource-group-label"}
                onClick={() => onSelectResourceGroup(group.id)}
                onDoubleClick={() => onOpenResourceGroup(group.id)}
                onKeyDown={(event) => {
                  if (event.key !== "Enter") return;
                  event.preventDefault();
                  openGroupRef.current(group.id);
                }}
                onFocus={() => cyRef.current?.getElementById(node.id).addClass("keyboard-focus")}
                onBlur={() => cyRef.current?.getElementById(node.id).removeClass("keyboard-focus")}
                aria-label={`${group.name}, ${group.resourceCount} resources in ${group.subscriptionName}. Press Enter to open.`}
              >
                <span className="graph-group-heading">
                  <span className="graph-group-icon"><img src={RESOURCE_GROUP_ICON} alt="" /></span>
                  <span className="graph-group-copy">
                    <strong>{group.name}</strong>
                    <small>{group.subscriptionName}</small>
                  </span>
                  {group.findingCount > 0 ? <em>{group.findingCount}</em> : null}
                </span>
                <span className="graph-group-stats">
                  <span><strong>{group.resourceCount}</strong> resources</span>
                  <i />
                  <span><strong>{group.connectedGroupCount}</strong> connected groups</span>
                </span>
                <span className="graph-group-types" aria-hidden="true">
                  {group.resourceTypes.slice(0, 4).map(({ azureType, count }) => {
                    const type = typeMap.get(azureType);
                    return type ? <span key={azureType}><img src={type.icon} alt="" /><b>{count}</b></span> : null;
                  })}
                  <span className="graph-group-open">Select group</span>
                </span>
              </button>
            );
          }
          const resource = node.resource;
          if (!resource) return null;
          const type = typeMap.get(resource.azureType);
          const selected = node.id === focusedNodeId;
          return (
            <button
              key={node.id}
              ref={(element) => {
                if (element) labelRefs.current.set(node.id, element);
                else labelRefs.current.delete(node.id);
              }}
              className={selected ? "graph-node-label selected" : "graph-node-label"}
              onClick={() => onSelectResource(resource.id)}
              onFocus={() => {
                cyRef.current?.getElementById(node.id).addClass("keyboard-focus");
              }}
              onBlur={() => cyRef.current?.getElementById(node.id).removeClass("keyboard-focus")}
              aria-label={`${resource.name}, ${type?.displayName ?? resource.azureType}`}
            >
              <span className="graph-node-icon">
                {type ? <img src={type.icon} alt="" /> : null}
              </span>
              <span className="graph-node-copy">
                <strong>{resource.name}</strong>
                <small>{type?.displayName ?? resource.azureType}</small>
              </span>
              {resource.findingCount > 0 ? <em>{resource.findingCount}</em> : null}
            </button>
          );
        })}
      </div>
      <div className="graph-controls-hint" aria-hidden="true">
        <span>{graph.level === "resource-groups" ? "Select a group · Enter to open" : "Drag nodes"}</span>
        <i /> <span>Drag canvas to pan</span><i /> <span>Scroll to zoom</span>
      </div>
      {graph.nodes.length === 0 ? (
        <div className="graph-empty-state" role="status">
          <img src={RESOURCE_GROUP_ICON} alt="" />
          <strong>No resources in this group</strong>
          <span>The stored snapshot contains the resource group, but no resource records belong to it.</span>
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
