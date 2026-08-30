import type { TopologyGraph, TopologyLink } from "../types";
import type { Placement } from "./topology-layout";

export type RelationshipLabelSide = "left" | "right" | "top" | "bottom";

export interface LinkPresentation {
  edgeId: string;
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

export interface LabelRect {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export interface RelationshipLabelPlacement {
  side: RelationshipLabelSide;
  x: number;
  y: number;
}

function stableCompare(left: string, right: string) {
  return left < right ? -1 : left > right ? 1 : 0;
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
  return `${link.label.charAt(0).toUpperCase()}${link.label.slice(1)}`;
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

export function connectorGeometries(
  presentations: readonly LinkPresentation[],
  positions: ReadonlyMap<string, Placement>,
): ConnectorGeometry[] {
  return presentations.map((presentation) => {
    const source = positions.get(presentation.sourceId);
    const target = positions.get(presentation.targetId);
    const deltaX = (target?.x ?? 0) - (source?.x ?? 0);
    const deltaY = (target?.y ?? 0) - (source?.y ?? 0);
    const horizontal = Math.abs(deltaX) >= Math.abs(deltaY);
    if (horizontal) {
      return {
        edgeId: presentation.edgeId,
        taxiDirection: "horizontal",
        taxiTurn: presentation.taxiTurn,
      };
    }

    return {
      edgeId: presentation.edgeId,
      taxiDirection: "vertical",
      taxiTurn: presentation.taxiTurn,
    };
  });
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
