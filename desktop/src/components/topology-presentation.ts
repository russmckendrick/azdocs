import type { TopologyGraph, TopologyLink } from "../types";
import type { Placement } from "./topology-layout";
import { stableCompare } from "../ordering";
import { capitalise } from "../format";

export type RelationshipLabelSide = "left" | "right" | "top" | "bottom";
export type ConnectorPortSide = RelationshipLabelSide;

export interface LinkPresentation {
  edgeId: string;
  sourcePortId: string;
  targetPortId: string;
  index: number;
  sourceId: string;
  targetId: string;
  label: string;
  kindClass: string;
  count: number;
  taxiTurn: string;
  sourceSlot: number;
  targetSlot: number;
}

export interface ActiveRelationshipLabel extends LinkPresentation {
  endpointId: string;
  endpointSlot: number;
}

export interface ConnectorGeometry {
  edgeId: string;
  taxiDirection: "horizontal" | "vertical";
  taxiTurn: string;
}

export interface ConnectorPort {
  side: ConnectorPortSide;
  offset: number;
}

export interface ConnectorPortAssignment extends ConnectorGeometry {
  sourceId: string;
  targetId: string;
  sourcePortId: string;
  targetPortId: string;
  sourcePort: ConnectorPort;
  targetPort: ConnectorPort;
}

export interface LabelRect {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export function connectorPortPosition(bounds: LabelRect, port: ConnectorPort): Placement {
  if (port.side === "left") {
    return { x: bounds.x1, y: bounds.y1 + (bounds.y2 - bounds.y1) * port.offset };
  }
  if (port.side === "right") {
    return { x: bounds.x2, y: bounds.y1 + (bounds.y2 - bounds.y1) * port.offset };
  }
  if (port.side === "top") {
    return { x: bounds.x1 + (bounds.x2 - bounds.x1) * port.offset, y: bounds.y1 };
  }
  return { x: bounds.x1 + (bounds.x2 - bounds.x1) * port.offset, y: bounds.y2 };
}

export interface RelationshipLabelPlacement {
  side: RelationshipLabelSide;
  x: number;
  y: number;
}

function linkKey(link: TopologyLink, index: number) {
  return [
    link.sourceId,
    link.targetId,
    link.kindClass,
    link.label,
    String(link.count).padStart(8, "0"),
    String(index).padStart(8, "0"),
  ].join("\0");
}

function slotsForEndpoint(
  links: Array<{ link: TopologyLink; index: number }>,
  endpoint: "sourceId" | "targetId",
) {
  const groups = new Map<string, Array<{ link: TopologyLink; index: number }>>();
  for (const entry of links) {
    groups.set(entry.link[endpoint], [...(groups.get(entry.link[endpoint]) ?? []), entry]);
  }

  const slots = new Map<number, number>();
  for (const entries of groups.values()) {
    entries.sort((left, right) => stableCompare(
      linkKey(left.link, left.index),
      linkKey(right.link, right.index),
    ));
    for (const [position, entry] of entries.entries()) {
      slots.set(entry.index, position - (entries.length - 1) / 2);
    }
  }
  return slots;
}

function displayLabel(graph: TopologyGraph, link: TopologyLink) {
  if (graph.level === "estate" && link.count > 1) return `${link.count} links`;
  if (!link.label) return "Relationship";
  return capitalise(link.label);
}

export function buildLinkPresentations(graph: TopologyGraph): LinkPresentation[] {
  const links = graph.links.map((link, index) => ({ link, index }));
  const sourceSlots = slotsForEndpoint(links, "sourceId");
  const targetSlots = slotsForEndpoint(links, "targetId");

  return links.map(({ link, index }) => {
    const sourceSlot = sourceSlots.get(index) ?? 0;
    const targetSlot = targetSlots.get(index) ?? 0;
    const turn = Math.min(70, Math.max(30, 50 + targetSlot * 8 + sourceSlot * 4));
    return {
      edgeId: `relationship-${index}`,
      sourcePortId: `relationship-${index}:source-port`,
      targetPortId: `relationship-${index}:target-port`,
      index,
      sourceId: link.sourceId,
      targetId: link.targetId,
      label: displayLabel(graph, link),
      kindClass: link.kindClass,
      count: link.count,
      taxiTurn: `${Number(turn.toFixed(1))}%`,
      sourceSlot,
      targetSlot,
    };
  });
}

function connectorSides(source: Placement, target: Placement) {
  const deltaX = target.x - source.x;
  const deltaY = target.y - source.y;
  if (Math.abs(deltaX) >= Math.abs(deltaY)) {
    return {
      sourceSide: (deltaX >= 0 ? "right" : "left") as ConnectorPortSide,
      targetSide: (deltaX >= 0 ? "left" : "right") as ConnectorPortSide,
      taxiDirection: "horizontal" as const,
    };
  }
  return {
    sourceSide: (deltaY >= 0 ? "bottom" : "top") as ConnectorPortSide,
    targetSide: (deltaY >= 0 ? "top" : "bottom") as ConnectorPortSide,
    taxiDirection: "vertical" as const,
  };
}

function portOffset(position: number, count: number) {
  // Keep ports clear of card corners while using enough of a compound frame's
  // boundary to turn a shared anchor into visibly separate approaches.
  return 0.15 + ((position + 1) / (count + 1)) * 0.7;
}

function projectedPortOffsets(endpoints: Array<{ oppositeAxis: number }>, axisStart: number, axisEnd: number) {
  const axisLength = Math.max(1, axisEnd - axisStart);
  const minimum = 0.15;
  const maximum = 0.85;
  const gap = Math.min(14 / axisLength, (maximum - minimum) / Math.max(1, endpoints.length - 1));
  const desired = endpoints.map((endpoint) => Math.min(
    maximum,
    Math.max(minimum, (endpoint.oppositeAxis - axisStart) / axisLength),
  ));
  const offsets = [...desired];
  for (let index = 1; index < offsets.length; index += 1) {
    offsets[index] = Math.max(offsets[index], offsets[index - 1] + gap);
  }

  const desiredCentre = desired.reduce((total, value) => total + value, 0) / desired.length;
  const packedCentre = offsets.reduce((total, value) => total + value, 0) / offsets.length;
  const lowerShift = minimum - offsets[0];
  const upperShift = maximum - offsets[offsets.length - 1];
  const shift = Math.min(upperShift, Math.max(lowerShift, desiredCentre - packedCentre));
  return offsets.map((offset) => offset + shift);
}

export function connectorPortAssignments(
  presentations: readonly LinkPresentation[],
  positions: ReadonlyMap<string, Placement>,
  bounds: ReadonlyMap<string, LabelRect> = new Map(),
): ConnectorPortAssignment[] {
  type Endpoint = {
    edgeId: string;
    endpoint: "source" | "target";
    nodeId: string;
    side: ConnectorPortSide;
    oppositeAxis: number;
  };
  type Working = {
    presentation: LinkPresentation;
    sourceSide: ConnectorPortSide;
    targetSide: ConnectorPortSide;
    taxiDirection: "horizontal" | "vertical";
  };

  const working: Working[] = [];
  const endpointGroups = new Map<string, Endpoint[]>();
  for (const presentation of presentations) {
    const source = positions.get(presentation.sourceId) ?? { x: 0, y: 0 };
    const target = positions.get(presentation.targetId) ?? { x: 0, y: 0 };
    const sides = connectorSides(source, target);
    working.push({ presentation, ...sides });
    const endpoints: Endpoint[] = [
      {
        edgeId: presentation.edgeId,
        endpoint: "source",
        nodeId: presentation.sourceId,
        side: sides.sourceSide,
        oppositeAxis: sides.sourceSide === "left" || sides.sourceSide === "right" ? target.y : target.x,
      },
      {
        edgeId: presentation.edgeId,
        endpoint: "target",
        nodeId: presentation.targetId,
        side: sides.targetSide,
        oppositeAxis: sides.targetSide === "left" || sides.targetSide === "right" ? source.y : source.x,
      },
    ];
    for (const endpoint of endpoints) {
      const key = `${endpoint.nodeId}\0${endpoint.side}`;
      endpointGroups.set(key, [...(endpointGroups.get(key) ?? []), endpoint]);
    }
  }

  const offsets = new Map<string, number>();
  for (const endpoints of endpointGroups.values()) {
    endpoints.sort((left, right) => left.oppositeAxis - right.oppositeAxis
      || stableCompare(left.edgeId, right.edgeId)
      || stableCompare(left.endpoint, right.endpoint));
    const first = endpoints[0];
    const endpointBounds = bounds.get(first.nodeId);
    const verticalSide = first.side === "left" || first.side === "right";
    const projected = endpointBounds
      ? projectedPortOffsets(
        endpoints,
        verticalSide ? endpointBounds.y1 : endpointBounds.x1,
        verticalSide ? endpointBounds.y2 : endpointBounds.x2,
      )
      : endpoints.map((_, position) => portOffset(position, endpoints.length));
    for (const [position, endpoint] of endpoints.entries()) {
      offsets.set(`${endpoint.edgeId}\0${endpoint.endpoint}`, projected[position]);
    }
  }

  return working.map(({ presentation, sourceSide, targetSide, taxiDirection }) => {
    const sourceOffset = offsets.get(`${presentation.edgeId}\0source`) ?? 0.5;
    const targetOffset = offsets.get(`${presentation.edgeId}\0target`) ?? 0.5;
    const turn = Math.min(70, Math.max(
      30,
      50 + (targetOffset - 0.5) * 32 + (sourceOffset - 0.5) * 18,
    ));
    return {
      edgeId: presentation.edgeId,
      sourceId: presentation.sourceId,
      targetId: presentation.targetId,
      sourcePortId: presentation.sourcePortId,
      targetPortId: presentation.targetPortId,
      sourcePort: { side: sourceSide, offset: sourceOffset },
      targetPort: { side: targetSide, offset: targetOffset },
      taxiDirection,
      taxiTurn: `${Number(turn.toFixed(1))}%`,
    };
  });
}

export function connectorGeometries(
  presentations: readonly LinkPresentation[],
  positions: ReadonlyMap<string, Placement>,
): ConnectorGeometry[] {
  return connectorPortAssignments(presentations, positions).map((assignment) => ({
    edgeId: assignment.edgeId,
    taxiDirection: assignment.taxiDirection,
    taxiTurn: assignment.taxiTurn,
  }));
}

export function activeRelationshipLabels(
  presentations: readonly LinkPresentation[],
  traceNodeId?: string,
): ActiveRelationshipLabel[] {
  if (!traceNodeId) return [];
  return presentations.flatMap((presentation) => {
    if (presentation.sourceId === traceNodeId) {
      return [{
        ...presentation,
        endpointId: presentation.targetId,
        endpointSlot: presentation.targetSlot,
      }];
    }
    if (presentation.targetId === traceNodeId) {
      return [{
        ...presentation,
        endpointId: presentation.sourceId,
        endpointSlot: presentation.sourceSlot,
      }];
    }
    return [];
  });
}

export function resolveTraceNode(
  focusedNodeId?: string,
  pointerNodeId?: string,
  selectedNodeId?: string,
) {
  return focusedNodeId ?? pointerNodeId ?? selectedNodeId;
}

export function relationshipLabelSide(
  trace: Placement,
  endpoint: Placement,
): RelationshipLabelSide {
  const deltaX = trace.x - endpoint.x;
  const deltaY = trace.y - endpoint.y;
  if (Math.abs(deltaX) >= Math.abs(deltaY)) return deltaX < 0 ? "right" : "left";
  return deltaY < 0 ? "bottom" : "top";
}

function labelCandidate(
  side: RelationshipLabelSide,
  endpoint: LabelRect,
  width: number,
  height: number,
  slot: number,
): RelationshipLabelPlacement {
  const centreX = (endpoint.x1 + endpoint.x2) / 2;
  const centreY = (endpoint.y1 + endpoint.y2) / 2;
  const slotOffset = slot * 24;
  const gap = 10;
  if (side === "left") return { side, x: endpoint.x1 - gap - width, y: centreY + slotOffset - height / 2 };
  if (side === "right") return { side, x: endpoint.x2 + gap, y: centreY + slotOffset - height / 2 };
  if (side === "top") return { side, x: centreX + slotOffset - width / 2, y: endpoint.y1 - gap - height };
  return { side, x: centreX + slotOffset - width / 2, y: endpoint.y2 + gap };
}

function placementRect(placement: RelationshipLabelPlacement, width: number, height: number): LabelRect {
  return {
    x1: placement.x,
    y1: placement.y,
    x2: placement.x + width,
    y2: placement.y + height,
  };
}

function rectsOverlap(left: LabelRect, right: LabelRect) {
  const clearance = 4;
  return left.x1 < right.x2 + clearance
    && left.x2 + clearance > right.x1
    && left.y1 < right.y2 + clearance
    && left.y2 + clearance > right.y1;
}

export function placeRelationshipLabel(
  trace: Placement,
  endpoint: LabelRect,
  size: { width: number; height: number },
  bounds: LabelRect,
  obstacles: readonly LabelRect[],
  slot: number,
): RelationshipLabelPlacement {
  const preferred = relationshipLabelSide(trace, {
    x: (endpoint.x1 + endpoint.x2) / 2,
    y: (endpoint.y1 + endpoint.y2) / 2,
  });
  const opposite: Record<RelationshipLabelSide, RelationshipLabelSide> = {
    left: "right",
    right: "left",
    top: "bottom",
    bottom: "top",
  };
  const perpendicular: Record<RelationshipLabelSide, RelationshipLabelSide[]> = {
    left: ["top", "bottom"],
    right: ["top", "bottom"],
    top: ["right", "left"],
    bottom: ["right", "left"],
  };
  const candidates = [preferred, ...perpendicular[preferred], opposite[preferred]]
    .map((side) => labelCandidate(side, endpoint, size.width, size.height, slot));
  const inside = (candidate: RelationshipLabelPlacement) => {
    const rect = placementRect(candidate, size.width, size.height);
    return rect.x1 >= bounds.x1
      && rect.y1 >= bounds.y1
      && rect.x2 <= bounds.x2
      && rect.y2 <= bounds.y2;
  };
  const clear = (candidate: RelationshipLabelPlacement) => {
    const rect = placementRect(candidate, size.width, size.height);
    return obstacles.every((obstacle) => !rectsOverlap(rect, obstacle));
  };
  return candidates.find((candidate) => inside(candidate) && clear(candidate))
    ?? candidates.find(inside)
    ?? {
      ...candidates[0],
      x: Math.min(bounds.x2 - size.width, Math.max(bounds.x1, candidates[0].x)),
      y: Math.min(bounds.y2 - size.height, Math.max(bounds.y1, candidates[0].y)),
    };
}
